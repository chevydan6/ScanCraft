"""Offline checks for trial assessment and owned-child cancellation, without hardware."""
import copy
import json
import os
from pathlib import Path
import signal
import select
import subprocess
import sys
import tempfile
import unittest

from check_stereo_soak import analyze


def report(count=1000, span=600., rejected=0):
    samples = [{'sequence': (2**32-5+i) % (2**32),
                'host_us': 100+round(i*span*1e6/(count-1)),
                'sensor_pts': (2**32-10+i*33337) % (2**32)} for i in range(count)]
    channel = {'samples': samples, 'valid_frames': count, 'rejected_buffers': rejected,
               'sequence_gaps': 0, 'metadata_missing': 0, 'duplicate_sensor_pts': 0}
    return {'left': copy.deepcopy(channel), 'right': copy.deepcopy(channel),
            'native_rust': True, 'wine_used': False, 'exact_sensor_pts_matches': count}


class AssessmentTests(unittest.TestCase):
    def test_duration_uses_accepted_sample_span_and_handles_wrap(self):
        data = report()
        result = analyze(data, 600.)
        self.assertEqual(result['channels']['left']['duration_gate'], 'PASS')
        self.assertEqual(result['channels']['left']['recomputed_accepted_sequence_gaps'], 0)
        data = report(span=599.999)
        data['elapsed_seconds'] = 650.
        data['left']['accepted_host_span_seconds'] = 999.
        self.assertEqual(analyze(data, 600.)['channels']['left']['duration_gate'], 'FAIL')

    def test_exact_rejection_threshold_and_separate_metadata_gate(self):
        self.assertEqual(analyze(report(count=999, rejected=1), 600.)['channels']['left']['buffer_rejection_proxy_gate'], 'FAIL')
        data = report(count=1000, rejected=1)
        data['left']['metadata_missing'] = 10
        result = analyze(data, 600.)['channels']['left']
        self.assertEqual(result['buffer_rejection_proxy_gate'], 'PASS')
        self.assertEqual(result['metadata_gate'], 'FAIL')
        self.assertEqual(result['rejection_fraction_of_accepted_plus_rejected_buffers'], 1/1001)

    def test_sequence_gaps_are_recomputed_separately(self):
        data = report()
        for sample in data['left']['samples'][20:]:
            sample['sequence'] = (sample['sequence']+7) % (2**32)
        data['left']['sequence_gaps'] = 7
        result = analyze(data, 600.)['channels']['left']
        self.assertEqual(result['recomputed_accepted_sequence_gaps'], 7)
        self.assertEqual(result['rejected_buffers'], 0)
        data['left']['sequence_gaps'] = 6
        with self.assertRaises(ValueError):
            analyze(data, 600.)

    def test_malformed_counts_and_accepted_orders_are_rejected(self):
        for mutation in ('count', 'negative', 'host', 'sequence', 'pts', 'pts_range'):
            data = report()
            left = data['left']
            if mutation == 'count':
                left['valid_frames'] += 1
            elif mutation == 'negative':
                left['rejected_buffers'] = -1
            elif mutation == 'host':
                left['samples'][1]['host_us'] = left['samples'][0]['host_us']
            elif mutation == 'sequence':
                left['samples'][1]['sequence'] = left['samples'][0]['sequence']
            elif mutation == 'pts':
                left['samples'][1]['sensor_pts'] = left['samples'][0]['sensor_pts']
            else:
                left['samples'][1]['sensor_pts'] = 2**32
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                analyze(data, 600.)

    def test_phase_counters_are_optional_for_legacy_and_checked_when_present(self):
        data=report(rejected=3)
        self.assertIsNone(analyze(data,600.)['channels']['left']['rejected_before_first_valid'])
        data['left'].update(rejected_before_first_valid=2,rejected_after_first_valid=1)
        self.assertEqual(analyze(data,600.)['channels']['left']['rejected_after_first_valid'],1)
        data['left']['rejected_after_first_valid']=2
        with self.assertRaises(ValueError): analyze(data,600.)
        del data['left']['rejected_after_first_valid']
        with self.assertRaises(ValueError): analyze(data,600.)

    def test_never_promotes_single_trial_to_full_qualification(self):
        result = analyze(report(), 600.)
        for key in ('exposure_synchronization_qualified', 'optical_content_qualified',
                    'physical_accuracy_qualified', 'sixty_minute_memory_gate_qualified',
                    'full_device_reliability_gate_qualified'):
            self.assertIs(result[key], False)

    def test_termination_stops_only_owned_fake_native_child(self):
        with tempfile.TemporaryDirectory(prefix='scancraft-soak-test-') as temp:
            root = Path(temp)
            binary = root/'fake-native'
            binary.write_text(f'#!{sys.executable}\nimport time\nwhile True: time.sleep(1)\n')
            binary.chmod(0o700)
            output = root/'result'
            runner = subprocess.Popen([sys.executable, str(Path(__file__).with_name('check_stereo_soak.py')),
                                       str(output), '--binary', str(binary), '--frames', '10', '--minimum-seconds', '1'],
                                      stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            child_pid = None
            child_start = None
            try:
                ready, _, _ = select.select([runner.stdout], [], [], 5)
                self.assertTrue(ready, "runner did not announce its owned child")
                started = json.loads(runner.stdout.readline())
                child_pid = started['native_pid']
                self.assertTrue(Path(f'/proc/{child_pid}').exists())
                child_start = Path(f'/proc/{child_pid}/stat').read_text().rsplit(')', 1)[1].split()[19]
                runner.send_signal(signal.SIGTERM)
                runner.communicate(timeout=10)
                self.assertNotEqual(runner.returncode, 0)
                self.assertFalse(Path(f'/proc/{child_pid}').exists())
                self.assertEqual(json.loads((output/'failure.json').read_text())['type'], 'KeyboardInterrupt')
            finally:
                if runner.poll() is None:
                    runner.kill()
                    runner.communicate(timeout=5)
                if child_pid is not None and Path(f'/proc/{child_pid}/stat').exists():
                    current_start = Path(f'/proc/{child_pid}/stat').read_text().rsplit(')', 1)[1].split()[19]
                    if current_start == child_start:
                        os.kill(child_pid, signal.SIGKILL)


if __name__ == '__main__':
    unittest.main()
