#!/usr/bin/env python3
"""Experimental CR-Scan Raptor Linux UVC transport. Not a scan application.

Uses the existing uvcvideo driver. No third party Python dependencies.
The default command inventories descriptors without sending vendor commands.
"""
import argparse
import ctypes as C
import fcntl
import json
import os
from pathlib import Path
import struct
import threading

VENDOR_GUID = bytes.fromhex('a15157a5c5f35e4a8d5a6854b8fa2716')


def video_node_kind(node):
    """Identify a V4L2 node without assuming adjacent device minor numbers."""
    fd = os.open(node, os.O_RDONLY | os.O_NONBLOCK | os.O_CLOEXEC | os.O_NOFOLLOW)
    try:
        capability = bytearray(104)
        fcntl.ioctl(fd, 0x80685600, capability, True)  # VIDIOC_QUERYCAP
        caps, device_caps = struct.unpack_from('<II', capability, 84)
        if caps & 0x80000000:  # V4L2_CAP_DEVICE_CAPS
            caps = device_caps
        if caps & 0x00800000:  # V4L2_CAP_META_CAPTURE
            return 'metadata'
        if caps & 1:  # V4L2_CAP_VIDEO_CAPTURE
            return 'capture'
        return 'other'
    finally:
        os.close(fd)


def descriptors(data):
    """Decode only UVC descriptors in the appropriate interface context."""
    result = {'interfaces': [], 'extension_units': [], 'formats': [], 'frames': []}
    interface = subclass = fmt = None
    offset = 0
    while offset < len(data):
        size = data[offset]
        if size < 2 or offset + size > len(data):
            raise ValueError(f'truncated descriptor at {offset}')
        d = data[offset:offset + size]
        if d[1] == 4 and size >= 9:
            interface, subclass, fmt = d[2], d[6] if d[5] == 14 else None, None
            result['interfaces'].append(dict(number=interface, alternate=d[3],
                                             usb_class=d[5], subclass=d[6]))
        elif d[1] == 0x24 and size >= 3:
            subtype = d[2]
            if subclass == 1 and subtype == 6 and size >= 24:
                pins = d[21]
                pos = 22 + pins
                if pos >= size:
                    raise ValueError('truncated extension unit')
                controls_size = d[pos]
                if pos + 1 + controls_size >= size:
                    raise ValueError('truncated control bitmap')
                bitmap = int.from_bytes(d[pos+1:pos+1+controls_size], 'little')
                result['extension_units'].append(dict(interface=interface, unit=d[3],
                    guid=d[4:20].hex(), selectors=[i+1 for i in range(controls_size*8)
                                                 if bitmap & (1 << i)]))
            elif subclass == 2 and subtype in (4, 6, 16):
                fmt = d[3]
                item = dict(interface=interface, index=fmt, subtype=subtype)
                if subtype in (4, 16) and size >= 21:
                    item['guid'] = d[5:21].hex()
                result['formats'].append(item)
            elif subclass == 2 and subtype in (5, 7) and size >= 26:
                count = d[25]
                if count and size < 26+count*4:
                    raise ValueError('truncated frame intervals')
                result['frames'].append(dict(interface=interface, format_index=fmt,
                    index=d[3], width=int.from_bytes(d[5:7], 'little'),
                    height=int.from_bytes(d[7:9], 'little'),
                    default_interval_100ns=int.from_bytes(d[21:25], 'little'),
                    intervals_100ns=[int.from_bytes(d[j:j+4], 'little')
                                    for j in range(26, 26+count*4, 4)]))
        offset += size
    return result


def discover():
    devices = []
    for path in sorted(Path('/sys/bus/usb/devices').glob('*')):
        try:
            if (path/'idVendor').read_text().strip() != '2bc5':
                continue
            if (path/'idProduct').read_text().strip() != '06a8':
                continue
            item = dict(usb_path=str(path.resolve()), product=(path/'product').read_text().strip(),
                        speed_mbps=(path/'speed').read_text().strip(),
                        descriptors=descriptors((path/'descriptors').read_bytes()), nodes=[])
            for node in sorted(Path('/sys/class/video4linux').glob('*')):
                if path.resolve() in node.resolve().parents:
                    interface = (node/'device').resolve()
                    entry = dict(path='/dev/'+node.name,
                        interface=int((interface/'bInterfaceNumber').read_text(), 16),
                        accessible=os.access('/dev/'+node.name, os.R_OK | os.W_OK))
                    try:
                        entry['kind'] = video_node_kind(entry['path'])
                    except OSError as error:
                        entry.update(kind='unknown', capability_error=str(error))
                    item['nodes'].append(entry)
            devices.append(item)
        except FileNotFoundError:
            continue
    return devices


class XuQuery(C.Structure):
    _fields_ = [('unit', C.c_uint8), ('selector', C.c_uint8), ('query', C.c_uint8),
                ('size', C.c_uint16), ('data', C.c_void_p)]


class RaptorTransport:
    """Vendor command channel on the Raptor depth-camera extension unit.

    Packet contents remain the caller's responsibility. This implements USB
    transport only, not calibration, frame decoding, or reconstruction.
    """
    def __init__(self, node):
        interface = (Path('/sys/class/video4linux')/Path(node).name/'device').resolve(strict=True)
        parent = interface.parent
        if ((parent/'idVendor').read_text().strip(),
            (parent/'idProduct').read_text().strip(),
            int((interface/'bInterfaceNumber').read_text(), 16)) != ('2bc5', '06a8', 0):
            raise ValueError('requires a Raptor depth-camera node')
        units = descriptors((parent/'descriptors').read_bytes())['extension_units']
        matches = [u for u in units if u['interface'] == 0 and u['guid'] == VENDOR_GUID.hex()]
        if len(matches) != 1 or not {1, 2, 3}.issubset(matches[0]['selectors']):
            raise ValueError('unexpected Raptor vendor extension unit')
        self.unit = matches[0]['unit']
        self.lock = threading.Lock()
        self.fd = os.open(node, os.O_RDWR | os.O_NONBLOCK | os.O_CLOEXEC | os.O_NOFOLLOW)
        try:
            fcntl.flock(self.fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BaseException:
            os.close(self.fd)
            raise

    def close(self):
        if self.fd is not None:
            os.close(self.fd)
            self.fd = None

    def __enter__(self):
        return self

    def __exit__(self, *args):
        self.close()

    def _query(self, selector, query, size, payload=b''):
        if selector not in (1, 2, 3) or not 1 <= size <= 1024 or len(payload) > size:
            raise ValueError('invalid XU request')
        buf = C.create_string_buffer(size)
        if payload:
            C.memmove(buf, payload, len(payload))
        arg = XuQuery(self.unit, selector, query, size, C.addressof(buf))
        # Linux _IOWR('u', 0x21, struct uvc_xu_control_query).
        request = 0xc0000000 | (C.sizeof(XuQuery) << 16) | (ord('u') << 8) | 0x21
        libc = C.CDLL(None, use_errno=True)
        if libc.ioctl(self.fd, C.c_ulong(request), C.byref(arg)) < 0:
            code = C.get_errno()
            raise OSError(code, os.strerror(code))
        return buf.raw

    def control_lengths(self):
        with self.lock:
            return {s: struct.unpack('<H', self._query(s, 0x85, 2))[0] for s in (1, 2, 3)}

    @staticmethod
    def channel(size):
        if not 1 <= size <= 1024:
            raise ValueError('packet length outside 1..1024')
        return (2, 64) if size <= 64 else ((1, 512) if size <= 512 else (3, 1024))

    def transact(self, packet, response_capacity=1024):
        """Forward a vendor packet with Windows SDK selector/padding semantics."""
        tx_selector, tx_size = self.channel(len(packet))
        rx_selector, rx_size = self.channel(response_capacity)
        with self.lock:
            for selector, expected in {(tx_selector, tx_size), (rx_selector, rx_size)}:
                actual = struct.unpack('<H', self._query(selector, 0x85, 2))[0]
                if actual != expected:
                    raise ValueError(f'control {selector}: expected {expected} bytes, got {actual}')
            self._query(tx_selector, 0x01, tx_size, packet)
            return self._query(rx_selector, 0x81, rx_size)

    @staticmethod
    def parse_response(response, opcode, request_id):
        if len(response) < 10:
            raise ValueError('truncated host-protocol response')
        magic, words, actual_opcode, actual_id, status = struct.unpack_from('<5H', response)
        if magic != 0x4252:
            raise ValueError(f'unexpected response magic {magic:#x}')
        if (actual_opcode, actual_id) != (opcode, request_id):
            raise ValueError('response does not match request')
        end = 8 + words * 2
        if end < 10 or end > len(response):
            raise ValueError('invalid response payload length')
        if status:
            raise OSError(f'device returned host-protocol status {status:#x}')
        return response[10:end]

    def protocol_version(self):
        # Orbbec host protocol read-only GET_PROTOCOL_VERSION (0xfe).
        # Decode the envelope only; retain version payload without guessing ABI.
        request_id = int.from_bytes(os.urandom(2), 'little')
        request = struct.pack('<4H', 0x4d47, 0, 0xfe, request_id)
        response = self.transact(request, 64)
        payload = self.parse_response(response, 0xfe, request_id)
        return dict(request=request.hex(), response=response.hex(), payload=payload.hex())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--probe-controls', action='store_true', help='query XU lengths; sends no vendor packets')
    parser.add_argument('--protocol-version', action='store_true', help='send read-only host-protocol version query')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    devices = discover()
    if args.probe_controls or args.protocol_version:
        for device in devices:
            for node in device['nodes']:
                if node['interface'] == 0 and node['kind'] == 'capture':
                    try:
                        with RaptorTransport(node['path']) as transport:
                            node['control_lengths'] = transport.control_lengths()
                            if args.protocol_version:
                                node['protocol_version'] = transport.protocol_version()
                    except (OSError, ValueError) as error:
                        node['probe_error'] = str(error)
    report = dict(status='experimental transport; scanning not implemented', devices=devices)
    content = json.dumps(report, indent=2)+'\n'
    if args.output:
        args.output.write_text(content)
    print(content, end='')


if __name__ == '__main__':
    main()
