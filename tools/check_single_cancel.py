#!/usr/bin/env python3
"""Hardware SIGINT/SIGTERM tests for single-camera validation and unpublished recording.
Raw image recordings exist only in an automatically removed private temporary directory.
"""
from evidence_redaction import public_replay_summary

import argparse
import hashlib
import json
from pathlib import Path
import signal
import subprocess
import tempfile
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, default=Path('target/release/scancraft-cli'))
    args = parser.parse_args()
    binary = args.binary.resolve()
    args.output.mkdir()
    def state():
        return [json.loads(subprocess.check_output([str(binary), 'stream-state', str(i)], text=True, timeout=7)) for i in (0, 2, 4)]
    def interrupted(_signal, _frame):
        raise KeyboardInterrupt('hardware test interrupted; stopping owned capture')
    signal.signal(signal.SIGTERM, interrupted)
    result = {'schema_version': 1, 'scope': 'Single-camera signal/state/publication tests; not optical or full recovery qualification.',
              'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'runner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'raw_image_payloads_retained_in_evidence': False, 'cases': [], 'completed_recordings': []}
    child = None
    try:
        with tempfile.TemporaryDirectory(prefix='scancraft-private-cancel-recording-') as private:
            private = Path(private)
            for interface in (0, 2, 4):
                for mode in ('validate', 'capture'):
                    for signum in (signal.SIGTERM, signal.SIGINT):
                        name = signal.Signals(signum).name
                        label = f'{interface}-{mode}-{name}'
                        before = state()
                        original = next(s for s in before if s['interface'] == interface)
                        candidates = (15, 20, 10) if interface == 4 else (10, 15, 30)
                        fps = next(rate for rate in candidates if original['frame_interval_denominator'] != rate*original['frame_interval_numerator'])
                        target = private/f'{label}.raptor'
                        partial = target.with_suffix('.raptor.partial')
                        command = [str(binary), mode, str(interface)] + ([str(target)] if mode == 'capture' else []) + ['30000', '--fps', str(fps)]
                        stderr_path = args.output/f'{label}.stderr'
                        with (args.output/f'{label}.stdout').open('x') as out, stderr_path.open('x') as err:
                            child = subprocess.Popen(command, stdout=out, stderr=err)
                            start = time.monotonic()
                            while 'validated frame 1/' not in stderr_path.read_text():
                                if child.poll() is not None:
                                    raise RuntimeError(f'{label}: acquisition ended before first valid buffer')
                                if time.monotonic()-start > 10:
                                    raise TimeoutError(f'{label}: no first valid buffer')
                                time.sleep(.05)
                            active = state()
                            active_target = next(s for s in active if s['interface'] == interface)
                            if active_target['frame_interval_denominator'] != fps*active_target['frame_interval_numerator']:
                                raise RuntimeError('active requested rate was not observed')
                            time.sleep(.15)
                            sent = time.monotonic()
                            child.send_signal(signum)
                            code = child.wait(timeout=12)
                            elapsed = time.monotonic()-sent
                        after = state()
                        stderr = stderr_path.read_text()
                        case = {'interface': interface, 'mode': mode, 'signal': name, 'owned_pid': child.pid,
                                'requested_fps': fps, 'first_valid_buffer_observed': True,
                                'before': before, 'active': active, 'after': after, 'native_exit_code': code,
                                'signal_to_exit_seconds': elapsed, 'reported_state_restored': after == before,
                                'cooperative_error_reported': code == 1 and name in stderr and 'cleanup requested' in stderr}
                        if mode == 'capture':
                            replay = subprocess.run([str(binary), 'replay', str(partial)], capture_output=True, text=True, timeout=10)
                            case.update(final_target_absent=not target.exists(), partial_exists=partial.exists(),
                                        partial_bytes=partial.stat().st_size if partial.exists() else 0,
                                        partial_replay_exit_code=replay.returncode, partial_replay_error=replay.stderr.strip(),
                                        partial_location_reported=str(partial) in stderr)
                        result['cases'].append(case)
                        (args.output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
                        if not case['reported_state_restored'] or not case['cooperative_error_reported']:
                            raise RuntimeError(f'{label}: signal cleanup/state restoration failed')
                        if mode == 'capture' and not (case['final_target_absent'] and case['partial_exists'] and case['partial_bytes'] > 64 and case['partial_replay_exit_code'] != 0 and case['partial_location_reported']):
                            raise RuntimeError(f'{label}: cancelled recording publication/integrity policy failed')
                        print(json.dumps({'case': label, 'restored': True, 'seconds': elapsed}), flush=True)
                        child = None
                # Verify successful final publication/replay still works after cancellation.
                target = private/f'complete-{interface}.raptor'
                completed = subprocess.run([str(binary), 'capture', str(interface), str(target), '3', '--fps', '10'], capture_output=True, text=True, check=True, timeout=15)
                replay = subprocess.run([str(binary), 'replay', str(target)], capture_output=True, text=True, check=True, timeout=10)
                summary = json.loads(completed.stdout)
                replay_summary = json.loads(replay.stdout)
                result['completed_recordings'].append({'interface': interface, 'summary': summary, 'replay': public_replay_summary(replay_summary),
                                                       'recording_bytes': target.stat().st_size})
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
