#!/usr/bin/env python3
"""Run a bounded native dual-IR structural acquisition trial, without storing images.
No firmware/calibration/projector writes. Duration/frame integrity are narrow gates;
this does not qualify exposure synchronization, optical content or reconstruction.
"""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import signal
import subprocess
import time


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def analyze(report, minimum_seconds):
    def integer(value, maximum=2**64-1):
        if type(value) is not int or not 0 <= value <= maximum:
            raise ValueError('invalid diagnostic integer')
        return value

    channels = {}
    for name in ('left', 'right'):
        stream = report[name]
        samples = stream['samples']
        phase_keys = ('rejected_before_first_valid', 'rejected_after_first_valid')
        if any(key in stream for key in phase_keys):
            if not all(key in stream for key in phase_keys):
                raise ValueError('incomplete rejection phase counters')
            for key in phase_keys:
                integer(stream[key])
            if sum(stream[key] for key in phase_keys) != stream['rejected_buffers']:
                raise ValueError('rejection phase counters do not sum to total')
        for key in ('valid_frames', 'rejected_buffers', 'sequence_gaps', 'metadata_missing', 'duplicate_sensor_pts'):
            integer(stream[key])
        for sample in samples:
            integer(sample['host_us'])
            integer(sample['sequence'], 2**32-1)
            integer(sample['sensor_pts'], 2**32-1)
        if len(samples) != stream['valid_frames'] or len(samples) < 2:
            raise ValueError('sample count mismatch or insufficient samples')
        monotonic = all(b['host_us'] > a['host_us'] for a, b in zip(samples, samples[1:]))
        if not monotonic:
            raise ValueError('accepted host timestamps are not strictly increasing')
        sequence_gaps = 0
        for a, b in zip(samples, samples[1:]):
            difference = (b['sequence'] - a['sequence']) % (2**32)
            if not 0 < difference < 2**31:
                raise ValueError('invalid accepted sequence order')
            sequence_gaps += difference - 1
            pts_difference = (b['sensor_pts']-a['sensor_pts']) % (2**32)
            if not 0 < pts_difference < 2**31:
                raise ValueError('invalid accepted sensor timestamp order')
        span = (samples[-1]['host_us'] - samples[0]['host_us']) / 1e6
        rejected = stream['rejected_buffers']
        valid = len(samples)
        proxy = rejected / (valid + rejected)
        channels[name] = {
            'valid_frames': valid, 'accepted_span_seconds': span,
            'accepted_buffer_rate_hz': (valid - 1) / span if span > 0 else None,
            'rejected_before_first_valid': stream.get('rejected_before_first_valid'),
            'rejected_after_first_valid': stream.get('rejected_after_first_valid'),
            'rejected_buffers': rejected, 'rejection_fraction_of_accepted_plus_rejected_buffers': proxy,
            'sequence_gaps': stream['sequence_gaps'],
            'recomputed_accepted_sequence_gaps': sequence_gaps,
            'metadata_missing': stream['metadata_missing'],
            'duplicate_sensor_pts': stream['duplicate_sensor_pts'],
            'accepted_host_timestamps_strictly_increasing': monotonic,
            'duration_gate': 'PASS' if span >= minimum_seconds else 'FAIL',
            'buffer_rejection_proxy_gate': 'PASS' if proxy < 0.001 else 'FAIL',
            'metadata_gate': 'PASS' if stream['metadata_missing'] == 0 else 'FAIL',
        }
        if sequence_gaps != stream['sequence_gaps']:
            raise ValueError('accepted sequence gap diagnostic mismatch')
    return {
        'scope': 'Single-session dual-IR native structural acquisition only; no saved image payloads.',
        'minimum_accepted_span_seconds': minimum_seconds,
        'channels': channels,
        'native_rust': report['native_rust'], 'wine_used': report['wine_used'],
        'exact_sensor_pts_matches': report['exact_sensor_pts_matches'],
        'exposure_synchronization_qualified': False,
        'optical_content_qualified': False, 'physical_accuracy_qualified': False,
        'sixty_minute_memory_gate_qualified': False,
        'full_device_reliability_gate_qualified': False,
        'limitations': [
            'Rejected V4L2 buffers are a proxy, not a count of physical corrupt exposures.',
            'The rejection denominator excludes metadata-missing buffers; their count/gate is separate.',
            'Sequence gaps are reported separately from rejected buffers; categories may overlap.',
            'PTS agreement does not establish exposure synchronization or correct image pairing.',
            'No raw images are saved; structural validation is performed by the native capture path.',
            'One connected-session run does not establish cold initialization/reconnection reliability.',
        ],
    }


def rss_kib(pid):
    try:
        for line in Path(f'/proc/{pid}/status').read_text().splitlines():
            if line.startswith('VmRSS:'):
                return int(line.split()[1])
    except FileNotFoundError:
        return None
    return None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, default=Path('target/release/scancraft-cli'))
    parser.add_argument('--frames', type=int, default=18100)
    parser.add_argument('--fps', type=int, default=30)
    parser.add_argument('--minimum-seconds', type=float, default=600.)
    args = parser.parse_args()
    if not 10 <= args.frames <= 30000 or args.fps not in (5, 10, 15, 20, 25, 30, 40, 50):
        parser.error('invalid native frame count or enumerated rate')
    if not 0 < args.minimum_seconds <= 6000:
        parser.error('minimum duration must be in (0, 6000] seconds')
    binary = args.binary.resolve()
    if not binary.is_file():
        parser.error('native release binary does not exist')
    args.output.mkdir()
    command = [str(binary), 'stereo-probe', str(args.frames), '--fps', str(args.fps)]
    metadata = {'schema_version': 1, 'command': command,
                'binary_sha256': sha(binary), 'runner_sha256': sha(Path(__file__)),
                'platform': platform.platform(), 'machine': platform.machine(),
                'started_utc': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()),
                'requested_minimum_accepted_span_seconds': args.minimum_seconds,
                'raw_image_payloads_saved': False}
    (args.output/'run.json').write_text(json.dumps(metadata, indent=2)+'\n')
    def interrupted(_signum, _frame):
        raise KeyboardInterrupt('trial interrupted; terminating the owned native child')

    signal.signal(signal.SIGTERM, interrupted)
    child = None
    measurements = []
    start = time.monotonic()
    try:
        with (args.output/'native.json').open('x') as out, (args.output/'progress.log').open('x') as err:
            child = subprocess.Popen(command, stdout=out, stderr=err)
            print(json.dumps({'native_pid': child.pid, 'output': str(args.output), 'state': 'running'}), flush=True)
            last_notice = -30.
            while child.poll() is None:
                elapsed = time.monotonic()-start
                measurements.append({'elapsed_seconds': elapsed, 'rss_kib': rss_kib(child.pid)})
                if elapsed-last_notice >= 30.:
                    print(json.dumps({'native_pid': child.pid, **measurements[-1], 'state': 'running'}), flush=True)
                    last_notice = elapsed
                # Bound unexpected stalls while allowing enumerated low-rate runs.
                if elapsed > args.frames / args.fps * 2 + 60:
                    raise TimeoutError('native trial exceeded its bounded runtime')
                time.sleep(2)
            exit_code = child.returncode
        metadata.update({'native_exit_code': exit_code, 'elapsed_wall_seconds': time.monotonic()-start,
                         'rss_samples': measurements,
                         'peak_sampled_rss_kib': max((m['rss_kib'] or 0 for m in measurements), default=0)})
        (args.output/'run.json').write_text(json.dumps(metadata, indent=2)+'\n')
        if exit_code:
            raise RuntimeError(f'native trial failed with exit {exit_code}; inspect progress.log')
        report = json.loads((args.output/'native.json').read_text())
        result = analyze(report, args.minimum_seconds)
        result['native_report_sha256'] = sha(args.output/'native.json')
        (args.output/'assessment.json').write_text(json.dumps(result, indent=2)+'\n')
        print(json.dumps({'state': 'complete', 'assessment': str(args.output/'assessment.json')}), flush=True)
    except BaseException as error:
        (args.output/'failure.json').write_text(json.dumps({'error': str(error), 'type': type(error).__name__,
            'elapsed_seconds': time.monotonic()-start, 'rss_samples': measurements}, indent=2)+'\n')
        raise
    finally:
        if child is not None and child.poll() is None:
            child.terminate()
            try:
                child.wait(timeout=12)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait(timeout=12)


if __name__ == '__main__':
    main()
