#!/usr/bin/env python3
"""Verify genuine CLI/stereo/native-GUI recording metadata; keep media private."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
from check_workbench_render import stop


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--cli', type=Path, default=Path('target/release/scancraft-cli'))
    parser.add_argument('--app', type=Path, default=Path('target/release/scancraft'))
    args = parser.parse_args()
    args.output.mkdir()
    cli, app = args.cli.resolve(), args.app.resolve()
    def run(*command):
        p = subprocess.run([str(cli), *map(str, command)], capture_output=True, text=True, timeout=30)
        if p.returncode:
            raise RuntimeError(p.stderr[-1000:])
        return json.loads(p.stdout)
    def states():
        return [run('stream-state', i) for i in (0, 2, 4)]
    def inspect(path):
        with path.open('rb') as f:
            assert f.read(8) == b'RAPTOR02'
            n = int.from_bytes(f.read(4), 'little')
            assert n <= 1024*1024
            f.read(4)
            header = json.loads(f.read(n))
        context = header['context']
        observed = context['controls']['capture_observations']
        assert context['device_identity'] is None and context['calibration_bytes'] is None
        assert context['calibration_status'] == 'not_loaded_or_verified'
        assert observed['read_only'] and not observed['per_frame_settings_verified'] and not observed['source_device_binding_verified']
        assert observed['requested_profile'] == header['profile']
        state = observed['stream_state']['value']
        assert state is not None and observed['stream_state']['error'] is None
        for key in ('interface', 'width', 'height', 'fourcc'):
            assert state[key] == header['profile'][key]
        assert state['frame_interval_numerator'] == 1 and state['frame_interval_denominator'] == header['profile']['fps']
        readings = observed['standard_controls']
        assert len(readings) == 8
        assert all((r['value'] is not None and r['error'] is None) or (r['value'] is None and r['error'] is not None) for r in readings)
        attributes = observed['usb_reported_attributes']
        assert len(attributes) == 9
        replay = run('replay', path, '--decode')
        assert replay['frames'] == replay['decoded_frames'] > 0 and replay['integrity'] == 'PASS'
        # Never retain descriptor values, context bytes, sensor images or private paths.
        return {'profile': header['profile'], 'frames': replay['frames'], 'decoded_frames': replay['decoded_frames'],
                'setup_phase': observed['sampling_phase'], 'reported_format_rate_matches_profile': True,
                'controls': [{'symbol': r['symbol'], 'value_present': r['value'] is not None, 'error_present': r['error'] is not None} for r in readings],
                'usb_attributes': [{'name': a['name'], 'value_present': a['observation']['value'] is not None, 'error_present': a['observation']['error'] is not None} for a in attributes],
                'identity_and_calibration_not_inferred': True, 'public_context_values_redacted': True}
    result = {'scope': 'Short genuine metadata/setup/recording checks; no physical calibration, synchronization or throughput qualification.',
              'cli_sha256': hashlib.sha256(cli.read_bytes()).hexdigest(), 'app_sha256': hashlib.sha256(app.read_bytes()).hexdigest(), 'cases': []}
    with tempfile.TemporaryDirectory(prefix='scancraft-metadata-', dir='evidence/private') as directory:
        private = Path(directory)
        for interface in (0, 2, 4):
            before = states()
            path = private/f'single-{interface}.raptor'
            capture = run('capture', interface, path, 3, '--fps', 10)
            checked = inspect(path)
            assert checked['frames'] == capture['frames'] == 3
            assert states() == before
            result['cases'].append({'entry': 'CLI single', 'interface': interface, 'metadata': checked, 'reported_camera_state_restored': True})
        before = states()
        stereo = private/'stereo'
        run('stereo-capture', 10, stereo, '--fps', 10)
        for name in ('left', 'right'):
            result['cases'].append({'entry': 'CLI stereo', 'side': name, 'metadata': inspect(stereo/f'{name}.raptor')})
        after = states()
        result['stereo_before'] = before
        result['stereo_after'] = after
        result['stereo_reported_camera_state_restored'] = after == before
        (args.output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
        assert after == before
        xvfb = Path(shutil.which('Xvfb') or Path.home()/'.cache/scancraft-test-tools/usr/bin/Xvfb')
        display = next(n for n in range(90, 200) if not Path(f'/tmp/.X11-unix/X{n}').exists() and not Path(f'/tmp/.X{n}-lock').exists())
        authority = private/'authority'
        authority.touch(mode=0o600)
        subprocess.run(['xauth', '-f', str(authority), 'add', f':{display}', 'MIT-MAGIC-COOKIE-1', os.urandom(16).hex()], check=True, capture_output=True)
        env = os.environ.copy()
        env.update({'DISPLAY': f':{display}', 'XAUTHORITY': str(authority), 'WINIT_UNIX_BACKEND': 'x11', 'XDG_SESSION_TYPE': 'x11', 'LIBGL_ALWAYS_SOFTWARE': '1'})
        env.pop('WAYLAND_DISPLAY', None)
        server = None
        try:
            with (private/'xvfb.log').open('w') as log:
                server = subprocess.Popen([str(xvfb), f':{display}', '-screen', '0', '1280x900x24', '-nolisten', 'tcp', '-auth', str(authority), '-noreset'], stdout=log, stderr=subprocess.STDOUT)
                for _ in range(50):
                    if server.poll() is not None:
                        raise RuntimeError('private display failed')
                    if subprocess.run(['xdpyinfo', '-display', f':{display}'], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0:
                        break
                    time.sleep(.1)
                else:
                    raise RuntimeError('private display did not become ready')
                for flag, label in (('--smoke-test', 'Color'), ('--smoke-test-ir', 'IR')):
                    before = states()
                    output = private/f'gui-{label}'
                    gui_env = {**env, 'SCANCRAFT_SMOKE_OUTPUT': str(output)}
                    p = subprocess.run([str(app), flag], env=gui_env, capture_output=True, text=True, timeout=30)
                    assert p.returncode == 0 and 'GUI_SMOKE decoded_image=true' in p.stdout
                    recordings = list(output.glob('*.raptor'))
                    assert len(recordings) == 1
                    checked = inspect(recordings[0])
                    assert checked['frames'] >= 20
                    assert states() == before
                    result['cases'].append({'entry': 'Native GUI private display', 'camera': label, 'metadata': checked, 'reported_camera_state_restored': True})
        finally:
            stop(server)
    result['raw_media_and_context_files_removed'] = True
    (args.output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps({'cases': len(result['cases']), 'metadata_and_decoded_replay': 'PASS', 'reported_camera_state_restored': True, 'raw_media_removed': True}))

if __name__ == '__main__':
    main()
