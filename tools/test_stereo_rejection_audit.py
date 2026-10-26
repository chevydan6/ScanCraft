import copy
import unittest
from check_stereo_rejection_audit import analyze, quantiles


def report():
    accepted = [{'host_us': x} for x in (0, 33333, 100000)]
    sample = {'sequence': 10, 'host_timestamp_us': 30000, 'video_bytes': 100,
              'metadata_bytes': 0, 'video_flags': 64, 'metadata_flags': 0,
              'raw_sensor_pts': 123, 'reason': 'short frame'}
    channel = {'samples': accepted, 'rejection_samples': [sample], 'rejected_buffers': 10}
    return {'left': copy.deepcopy(channel), 'right': copy.deepcopy(channel)}


class AuditTests(unittest.TestCase):
    def test_nearest_rank_is_explicit_and_empty_is_not_zero(self):
        self.assertIsNone(quantiles([]))
        self.assertEqual(quantiles([9, 1, 3, 2])['p50'], 2)
        self.assertEqual(quantiles([9, 1, 3, 2])['p99'], 9)

    def test_sample_scope_and_raw_pts_diagnostic_are_not_qualification(self):
        data = report()
        data['right']['rejection_samples'][0]['host_timestamp_us'] += 14
        result = analyze(data, 1000)
        left = result['channels']['left']
        self.assertEqual(left['accepted_interval_us_nearest_rank_quantiles']['max'], 66667)
        self.assertEqual(left['sample_signed_payload_deficit_counts'], {'900': 1})
        self.assertTrue(left['rejection_samples_truncated'])
        self.assertEqual(result['unique_raw_rejection_pts_intersection_count'], 1)
        self.assertEqual(result['unique_raw_rejection_pts_intersection'][0]['right_minus_left_host_us'], 14)
        self.assertIs(result['transport_cause_identified'], False)
        self.assertIs(result['exposure_synchronization_verified'], False)

    def test_duplicate_or_missing_raw_pts_cannot_make_unique_pair(self):
        data = report()
        data['left']['rejection_samples'] *= 2
        result = analyze(data)
        self.assertEqual(result['unique_raw_rejection_pts_intersection_count'], 0)
        self.assertEqual(result['channels']['left']['ambiguous_duplicate_raw_pts_keys'], 1)
        data = report()
        data['right']['rejection_samples'][0]['raw_sensor_pts'] = None
        self.assertEqual(analyze(data)['unique_raw_rejection_pts_intersection_count'], 0)

    def test_ongoing_sample_is_independent_and_unknown_legacy_is_not_empty_success(self):
        data=report()
        with self.assertRaises(ValueError): analyze(data,ongoing=True)
        for channel in data.values():
            channel['ongoing_rejection_samples']=copy.deepcopy(channel['rejection_samples'])
            channel['rejected_after_first_valid']=2
            channel['ongoing_rejection_samples'][0].update(raw_first_uvc_flags=4,metadata_pts_parse_error=None)
        result=analyze(data,ongoing=True)
        left=result['channels']['left']
        self.assertEqual(result['sample_set'],'after_first_valid')
        self.assertEqual(left['sample_population_rejected_buffers'],2)
        self.assertEqual(left['sample_metadata_pts_parse_successes'],1)
        self.assertEqual(left['sample_metadata_pts_parse_not_recorded'],0)
        self.assertEqual(left['sample_video_error_flags'],1)
        self.assertEqual(analyze(report())['channels']['left']['sample_metadata_pts_parse_not_recorded'],1)

    def test_invalid_counts_or_accepted_order_are_rejected(self):
        data = report()
        data['left']['rejected_buffers'] = 0
        with self.assertRaises(ValueError):
            analyze(data)
        data = report()
        data['left']['samples'][1]['host_us'] = 0
        with self.assertRaises(ValueError):
            analyze(data)


if __name__ == '__main__':
    unittest.main()
