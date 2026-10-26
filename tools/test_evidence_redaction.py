import copy
import unittest
from evidence_redaction import public_replay_summary

class RedactionTests(unittest.TestCase):
    def test_context_is_omitted_without_mutating_replay_or_losing_integrity_counts(self):
        summary = {'frames': 3, 'decoded_frames': 3, 'integrity': 'PASS', 'context': {'device_identity': 'private serial', 'calibration_bytes': [1, 2], 'controls': {'capture_observations': {'usb_reported_attributes': ['private descriptor']}}}}
        before = copy.deepcopy(summary)
        result = public_replay_summary(summary)
        self.assertNotIn('context', result)
        self.assertNotIn('private', repr(result))
        self.assertEqual(result['decoded_frames'], 3)
        self.assertTrue(result['capture_observations_present'])
        self.assertEqual(summary, before)

    def test_unknown_legacy_context_does_not_invent_observations(self):
        result = public_replay_summary({'frames': 2, 'context': None})
        self.assertFalse(result['capture_context_present'])
        self.assertFalse(result['capture_observations_present'])
