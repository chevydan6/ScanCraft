#!/usr/bin/env python3
"""Summarize accepted timing and the bounded rejection sample from native stereo-probe.
No exposure-pair or transport-causality claim follows from matching raw PTS fields.
"""
import argparse
from collections import Counter
import hashlib
import json
import math
from pathlib import Path


def quantiles(values):
    values = sorted(values)
    if not values:
        return None
    return {name: values[max(0, math.ceil(fraction*len(values))-1)]
            for name, fraction in [('min', 0.), ('p50', .5), ('p95', .95), ('p99', .99), ('max', 1.)]}


def analyze(report, expected_video_bytes=1920*1200, ongoing=False):
    result = {'scope': 'Accepted timing and bounded rejection samples; not all rejection events.',
              'sample_set': 'after_first_valid' if ongoing else 'first_overall',
              'expected_video_bytes': expected_video_bytes,
              'exposure_synchronization_verified': False, 'transport_cause_identified': False,
              'channels': {}}
    unique = {}
    for name in ('left', 'right'):
        channel = report[name]
        accepted = channel['samples']
        if ongoing and ('ongoing_rejection_samples' not in channel or 'rejected_after_first_valid' not in channel):
            raise ValueError('ongoing rejection sample was not recorded in this report')
        samples = channel['ongoing_rejection_samples'] if ongoing else channel['rejection_samples']
        population = channel['rejected_after_first_valid'] if ongoing else channel['rejected_buffers']
        if len(samples) > population:
            raise ValueError('more rejection samples than rejected buffers')
        deltas = [b['host_us']-a['host_us'] for a, b in zip(accepted, accepted[1:])]
        if any(delta <= 0 for delta in deltas):
            raise ValueError('nonmonotonic accepted host timestamps')
        pts_counts = Counter(s['raw_sensor_pts'] for s in samples if s['raw_sensor_pts'] is not None)
        unique[name] = {s['raw_sensor_pts']: s for s in samples
                        if s['raw_sensor_pts'] is not None and pts_counts[s['raw_sensor_pts']] == 1}
        result['channels'][name] = {
            'accepted_interval_us_nearest_rank_quantiles': quantiles(deltas),
            'reported_rejected_buffers': channel['rejected_buffers'],
            'retained_rejection_samples': len(samples),
            'rejection_samples_truncated': len(samples) < population,
            'sample_population_rejected_buffers': population,
            'sample_video_error_flags': sum(bool(s['video_flags'] & 0x40) for s in samples),
            'sample_metadata_error_flags': sum(bool(s['metadata_flags'] & 0x40) for s in samples),
            'sample_raw_uvc_flags_counts': dict(Counter(str(s.get('raw_first_uvc_flags', 'not_recorded')) for s in samples)),
            'sample_metadata_pts_parse_successes': sum('metadata_pts_parse_error' in s and s['metadata_pts_parse_error'] is None for s in samples),
            'sample_metadata_pts_parse_errors': dict(Counter(s['metadata_pts_parse_error'] for s in samples if s.get('metadata_pts_parse_error') is not None)),
            'sample_metadata_pts_parse_not_recorded': sum('metadata_pts_parse_error' not in s for s in samples),
            'sample_reason_counts': dict(Counter(s['reason'] for s in samples)),
            'sample_video_bytes_counts': dict(sorted(Counter(str(s['video_bytes']) for s in samples).items())),
            'sample_metadata_bytes_counts': dict(sorted(Counter(str(s['metadata_bytes']) for s in samples).items())),
            'sample_signed_payload_deficit_counts': dict(sorted(Counter(str(expected_video_bytes-s['video_bytes']) for s in samples).items())),
            'sample_video_flags_counts': dict(Counter(str(s['video_flags']) for s in samples)),
            'sample_metadata_flags_counts': dict(Counter(str(s['metadata_flags']) for s in samples)),
            'sample_host_span_seconds': (samples[-1]['host_timestamp_us']-samples[0]['host_timestamp_us'])/1e6 if len(samples)>1 else None,
            'ambiguous_duplicate_raw_pts_keys': sum(count>1 for count in pts_counts.values()),
        }
    common = sorted(set(unique['left']) & set(unique['right']))
    result['unique_raw_rejection_pts_intersection_count'] = len(common)
    result['unique_raw_rejection_pts_intersection'] = [
        {'raw_sensor_pts': pts, 'left_sequence': unique['left'][pts]['sequence'],
         'right_sequence': unique['right'][pts]['sequence'],
         'right_minus_left_host_us': unique['right'][pts]['host_timestamp_us']-unique['left'][pts]['host_timestamp_us']}
        for pts in common]
    result['limitations'] = [
        'Rejection reasons, lengths and flags describe retained samples only, not the complete rejected population.',
        'Raw PTS fields in rejected metadata have not passed the accepted-frame metadata checks.',
        'Shared raw PTS is a diagnostic coincidence, not verified exposure correspondence or a root cause.',
        'Accepted host intervals describe buffer delivery timestamps, not preview latency or optical exposure duration.',
    ]
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('report', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--ongoing', action='store_true', help='inspect independent post-first-valid rejection sample')
    args = parser.parse_args()
    if args.report.stat().st_size > 64*1024*1024:
        parser.error('diagnostic report exceeds 64 MiB')
    raw = args.report.read_bytes()
    result = analyze(json.loads(raw), ongoing=args.ongoing)
    result['native_report_sha256'] = hashlib.sha256(raw).hexdigest()
    result['analyzer_sha256'] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    with args.output.open('x') as out:
        json.dump(result, out, indent=2)
        out.write('\n')


if __name__ == '__main__':
    main()
