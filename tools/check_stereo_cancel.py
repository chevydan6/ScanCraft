#!/usr/bin/env python3
"""Hardware test: SIGTERM/SIGINT during native dual-IR capture restore reported format/rate.
Uses no projector/firmware/calibration controls; only signals the owned child.
"""
import argparse
import hashlib
import json
from pathlib import Path
import signal
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, default=Path('target/release/scancraft-cli'))
    args = parser.parse_args()
    binary = args.binary.resolve()
    args.output.mkdir()
    def state():
        return [json.loads(subprocess.check_output([str(binary), 'stream-state', str(i)], text=True, timeout=7)) for i in (0, 2)]
    def interrupted(_signum, _frame):
        raise KeyboardInterrupt('cancellation test interrupted; stopping owned child')
    signal.signal(signal.SIGTERM, interrupted)
    result = {'schema_version': 1, 'scope': 'Two real native dual-IR cancellations; no illumination/recording/cold-reconnect qualification.',
              'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'runner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), 'cases': []}
    child = None
    try:
        for signum in (signal.SIGTERM, signal.SIGINT):
            label = signal.Signals(signum).name
            before = state()
            fps = next(rate for rate in (15, 30, 10) if all(s['frame_interval_denominator'] != rate*s['frame_interval_numerator'] for s in before))
            with (args.output/f'{label}.stdout').open('x') as out, (args.output/f'{label}.stderr').open('x') as err:
                child = subprocess.Popen([str(binary), 'stereo-probe', '30000', '--fps', str(fps)], stdout=out, stderr=err)
                started = time.monotonic()
                while True:
                    if child.poll() is not None:
                        raise RuntimeError('native capture exited before cancellation')
                    active = state()
                    if all(s['frame_interval_denominator'] == fps*s['frame_interval_numerator'] and s['fourcc'] == list(b'GREY') for s in active):
                        break
                    if time.monotonic()-started > 10:
                        raise TimeoutError('requested active IR format/rate was not observed')
                    time.sleep(.1)
                time.sleep(.5)
                sent = time.monotonic()
                child.send_signal(signum)
                code = child.wait(timeout=12)
                latency = time.monotonic()-sent
            stderr = (args.output/f'{label}.stderr').read_text()
            after = state()
            case = {'signal': label, 'owned_pid': child.pid, 'requested_fps': fps, 'before': before, 'active': active,
                    'after': after, 'native_exit_code': code, 'signal_to_exit_seconds': latency,
                    'reported_format_and_rate_restored': after == before,
                    'cooperative_error_reported': code == 1 and label in stderr and 'cleanup requested' in stderr}
            result['cases'].append(case)
            (args.output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
            if not case['reported_format_and_rate_restored'] or not case['cooperative_error_reported']:
                raise RuntimeError('cancellation or reported state restoration failed; inspect result.json')
            print(json.dumps({'signal': label, 'state_restored': True, 'signal_to_exit_seconds': latency}), flush=True)
            child = None
        with (args.output/'reopen.json').open('x') as out, (args.output/'reopen.stderr').open('x') as err:
            subprocess.run([str(binary), 'stereo-probe', '10', '--fps', '30'], stdout=out, stderr=err, check=True, timeout=12)
        reopened = json.loads((args.output/'reopen.json').read_text())
        result['reopen_valid_buffers'] = {k: reopened[k]['valid_frames'] for k in ('left', 'right')}
        result['reopen_rejected_buffers'] = {k: reopened[k]['rejected_buffers'] for k in ('left', 'right')}
        result['completed'] = True
        (args.output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
    except BaseException as error:
        (args.output/'failure.json').write_text(json.dumps({'error': str(error), 'type': type(error).__name__, 'partial_result': result}, indent=2)+'\n')
        raise
    finally:
        if child is not None and child.poll() is None:
            child.terminate()
            try:
                child.wait(timeout=12)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait(timeout=5)


if __name__ == '__main__':
    main()
