#!/usr/bin/env python3
"""Recover genuine cancelled captures privately; retain only diagnostics and hashes."""
from evidence_redaction import public_replay_summary

import argparse
import hashlib
import json
from pathlib import Path
import signal
import subprocess
import tempfile
import time


def digest(path, limit=None):
    h = hashlib.sha256()
    with path.open('rb') as source:
        while limit is None or limit:
            block = source.read(65536 if limit is None else min(65536, limit))
            if not block:
                break
            h.update(block)
            if limit is not None:
                limit -= len(block)
    if limit:
        raise ValueError('file shorter than validated prefix')
    return h.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, default=Path('target/release/scancraft-cli'))
    args = parser.parse_args()
    binary = args.binary.resolve()
    args.output.mkdir()
    result = {'schema_version': 1, 'scope': 'Three genuine cancelled-capture prefix recoveries only; not geometry qualification.',
              'binary_sha256': digest(binary), 'runner_sha256': digest(Path(__file__)),
              'raw_image_payloads_retained_in_evidence': False, 'cases': []}
    child = None
    def interrupted(_signal, _frame):
        raise KeyboardInterrupt('test interrupted; stopping owned native capture')
    signal.signal(signal.SIGTERM, interrupted)
    try:
        with tempfile.TemporaryDirectory(prefix='scancraft-private-recovery-') as private:
            private = Path(private)
            for interface in (0, 2, 4):
                target = private/f'{interface}.raptor'
                partial = target.with_suffix('.raptor.partial')
                log = args.output/f'{interface}-capture.stderr'
                with log.open('x') as err, (args.output/f'{interface}-capture.stdout').open('x') as out:
                    child = subprocess.Popen([str(binary), 'capture', str(interface), str(target), '30000', '--fps', '10'], stderr=err, stdout=out)
                    started = time.monotonic()
                    while 'validated frame 1/' not in log.read_text():
                        if child.poll() is not None or time.monotonic()-started>10:
                            raise RuntimeError('no first validated buffer before cancellation')
                        time.sleep(.05)
                    time.sleep(.2)
                    child.send_signal(signal.SIGTERM)
                    code = child.wait(timeout=12)
                if code != 1 or target.exists() or not partial.exists():
                    raise RuntimeError('expected cancelled unpublished recording')
                child = None
                before = digest(partial)
                bundle = private/f'recovery-{interface}'
                run = subprocess.run([str(binary), 'recover-recording', str(partial), '1000', str(bundle)], capture_output=True, text=True, check=True, timeout=20)
                summary = json.loads(run.stdout)
                details = json.loads((bundle/'report.json').read_text())
                recording = bundle/'recovered.raptor'
                replay = subprocess.run([str(binary), 'replay', str(recording), '--decode'], capture_output=True, text=True, check=True, timeout=20)
                replay = json.loads(replay.stdout)
                report = details['recovery']
                prefix = report['validated_prefix_end_bytes']
                prefix_equal = digest(partial, prefix) == digest(recording, prefix)
                source_unchanged = before == digest(partial)
                if not (prefix_equal and source_unchanged and recording.stat().st_size == prefix+12 and replay['frames'] == report['recovered_frames'] == replay['decoded_frames'] and replay['frames'] > 0 and not report['source_complete']):
                    raise RuntimeError('recovery prefix, source or decoded replay mismatch')
                repeated = subprocess.run([str(binary), 'recover-recording', str(partial), '1000', str(bundle)], capture_output=True, text=True, timeout=20)
                if repeated.returncode == 0:
                    raise RuntimeError('recovery overwrote an existing bundle')
                case = {'interface': interface, 'source_bytes': partial.stat().st_size, 'source_sha256': before,
                        'source_unchanged': source_unchanged, 'validated_prefix_byte_exact': prefix_equal,
                        'recovered_file_has_only_prefix_plus_footer': True, 'overwrite_rejected': True,
                        'recovery_report': details, 'decoded_replay': public_replay_summary(replay)}
                result['cases'].append(case)
                (args.output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
                print(json.dumps({'interface': interface, 'recovered_frames': replay['frames'], 'decoded_frames': replay['decoded_frames'], 'source_unchanged': True}), flush=True)
            result['completed'] = True
        result['private_recording_directory_removed'] = not private.exists()
        (args.output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
    except BaseException as error:
        (args.output/'failure.json').write_text(json.dumps({'error': str(error), 'partial_result': result}, indent=2)+'\n')
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
