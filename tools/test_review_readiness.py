#!/usr/bin/env python3
"""Offline synthetic review-evidence regression tests; no GitHub writes or tokens."""
import copy
import unittest
from review_readiness import assess


class ReadinessTests(unittest.TestCase):
    def setUp(self):
        self.pr = {'headRefOid': 'new', 'state': 'OPEN', 'isDraft': False}
        self.review = {'id': 1, 'user': {'login': 'coderabbitai[bot]'},
                       'commit_id': 'new', 'state': 'COMMENTED', 'submitted_at': 'time',
                       'body': '<!-- coderabbit-review-publication v1 publication=x attempt=y batch=1/1 -->'}
        self.threads = {'headRefOid': 'new', 'reviewThreads': {
            'pageInfo': {'hasNextPage': False}, 'nodes': []}}

    def run_assessment(self, reviews=None, after=None, threads=None):
        return assess(self.pr, after or self.pr, reviews if reviews is not None else [self.review],
                      threads or self.threads)

    def test_green_or_skipped_check_without_formal_review_never_proves_coverage(self):
        self.pr['statusCheckRollup'] = [{'name': 'CodeRabbit', 'conclusion': 'SUCCESS'}]
        self.assertFalse(self.run_assessment(reviews=[])['coverage_and_conversations_ready'])
        self.review['commit_id'] = 'old'
        self.assertFalse(self.run_assessment()['coverage_and_conversations_ready'])

    def test_current_completed_review_is_coverage_not_merge_authorization(self):
        report = self.run_assessment()
        self.assertTrue(report['coverage_and_conversations_ready'])
        self.assertFalse(report['merge_authorized'])

    def test_spoofed_pending_dismissed_or_unknown_format_review_fails_closed(self):
        for key, value in [('user', {'login': 'another-user'}), ('state', 'PENDING'),
                           ('state', 'DISMISSED'), ('body', 'Review skipped'), ('submitted_at', None)]:
            candidate = copy.deepcopy(self.review)
            candidate[key] = value
            self.assertFalse(self.run_assessment(reviews=[candidate])['coverage_and_conversations_ready'])

    def test_draft_closed_or_changed_head_blocks(self):
        for key, value in [('isDraft', True), ('state', 'CLOSED'), ('headRefOid', 'changed')]:
            after = dict(self.pr, **{key: value})
            self.assertFalse(self.run_assessment(after=after)['coverage_and_conversations_ready'])

    def test_unresolved_even_outdated_conversation_blocks(self):
        self.threads['reviewThreads']['nodes'] = [{'isResolved': False, 'isOutdated': True,
            'comments': {'pageInfo': {'hasNextPage': False}, 'nodes': [{'author': {'login': 'developer'}}]}}]
        self.assertFalse(self.run_assessment()['coverage_and_conversations_ready'])
        self.threads['reviewThreads']['nodes'][0]['isResolved'] = True
        self.assertTrue(self.run_assessment()['coverage_and_conversations_ready'])

    def test_incomplete_or_mismatched_snapshot_fails_closed(self):
        self.threads['reviewThreads']['pageInfo']['hasNextPage'] = True
        self.assertFalse(self.run_assessment()['coverage_and_conversations_ready'])
        self.threads['reviewThreads']['pageInfo']['hasNextPage'] = False
        self.threads['headRefOid'] = 'other'
        self.assertFalse(self.run_assessment()['coverage_and_conversations_ready'])

    def test_all_batches_of_one_attempt_are_required(self):
        first = dict(self.review, body=self.review['body'].replace('batch=1/1', 'batch=1/2'))
        second = dict(first, id=2, body=first['body'].replace('batch=1/2', 'batch=2/2'))
        self.assertFalse(self.run_assessment(reviews=[first])['coverage_and_conversations_ready'])
        self.assertTrue(self.run_assessment(reviews=[first, second])['coverage_and_conversations_ready'])
        second['body'] = second['body'].replace('attempt=y', 'attempt=other')
        self.assertFalse(self.run_assessment(reviews=[first, second])['coverage_and_conversations_ready'])

    def test_conflicting_totals_in_one_publication_attempt_are_not_complete(self):
        conflicting = dict(self.review, id=2,
                           body=self.review['body'].replace('batch=1/1', 'batch=1/2'))
        self.assertFalse(self.run_assessment(reviews=[self.review, conflicting])['coverage_and_conversations_ready'])

    def test_deleted_user_and_incomplete_comments_cannot_produce_false_readiness(self):
        self.review['user'] = None
        self.assertFalse(self.run_assessment()['coverage_and_conversations_ready'])
        self.review['user'] = {'login': 'coderabbitai[bot]'}
        self.threads['reviewThreads']['nodes'] = [{'isResolved': True, 'isOutdated': False,
            'comments': {'pageInfo': {'hasNextPage': True}, 'nodes': []}}]
        self.assertFalse(self.run_assessment()['coverage_and_conversations_ready'])

    def test_changes_requested_not_silently_superseded(self):
        self.review['state'] = 'CHANGES_REQUESTED'
        later = dict(self.review, state='COMMENTED', id=2)
        self.assertFalse(self.run_assessment(reviews=[self.review, later])['coverage_and_conversations_ready'])


class CleanSummaryTests(unittest.TestCase):
    def setUp(self):
        ReadinessTests.setUp(self)
        self.pr['url'] = 'https://github.com/owner/repo/pull/1'
        self.comment = {'id': 10, 'user': {'login': 'coderabbitai[bot]'},
                        'html_url': self.pr['url']+'#issuecomment-10',
                        'body': ('<!-- coderabbit-ai-generated:v1 -->\n'
                                 '<!-- recent_review_start -->\n'
                                 'No actionable comments were generated in the recent review.\n'
                                 '- **Commit**: [new](<https://github.com/owner/repo/commit/new>)\n'
                                 '- **Run ID**: `e80421b6-3071-4107-b7a9-6c6f29fbf176`\n'
                                 '<!-- recent_review_end -->\n'
                                 '<!-- final_review_risk_coverage:{"sourceCommitId":"new",'
                                 '"coveredCommitId":"new","kind":"reviewed"} -->')}

    def summary_report(self):
        return assess(self.pr, self.pr, [], self.threads, [self.comment])

    def test_clean_summary_proves_coverage_without_formal_review_but_not_merge_authority(self):
        report = self.summary_report()
        self.assertTrue(report['coverage_and_conversations_ready'])
        self.assertEqual(report['known_current_head_reviews'], [])
        self.assertEqual(report['known_current_head_clean_summaries'][0]['id'], 10)
        self.assertFalse(report['merge_authorized'])

    def test_wrong_actor_head_repo_run_or_completion_cannot_prove_coverage(self):
        original = copy.deepcopy(self.comment)
        for old, new in [('"coveredCommitId":"new"', '"coveredCommitId":"old"'),
                         ('"sourceCommitId":"new"', '"sourceCommitId":"old"'),
                         ('"kind":"reviewed"', '"kind":"triaged"'),
                         ('/owner/repo/commit/', '/other/repo/commit/'),
                         ('**Run ID**:', '**Not a run**:'),
                         ('No actionable comments were generated in the recent review.', 'Review skipped'),
                         ('<!-- recent_review_end -->', '<!-- unfinished -->')]:
            self.comment = copy.deepcopy(original)
            self.comment['body'] = self.comment['body'].replace(old, new)
            self.assertFalse(self.summary_report()['coverage_and_conversations_ready'], (old, new))
        self.comment = original
        self.comment['user'] = {'login': 'owner'}
        self.assertFalse(self.summary_report()['coverage_and_conversations_ready'])

    def test_ambiguous_or_invalid_coverage_marker_fails_closed(self):
        self.comment['body'] += '\n<!-- final_review_risk_coverage:{} -->'
        self.assertFalse(self.summary_report()['coverage_and_conversations_ready'])
        self.comment['body'] = self.comment['body'].split('<!-- final_review_risk_coverage:', 1)[0]+'<!-- final_review_risk_coverage:INVALID -->'
        self.assertFalse(self.summary_report()['coverage_and_conversations_ready'])

    def test_clean_summary_does_not_bypass_unresolved_conversation(self):
        self.threads['reviewThreads']['nodes'] = [{'isResolved': False, 'isOutdated': True,
            'comments': {'pageInfo': {'hasNextPage': False}, 'nodes': []}}]
        self.assertFalse(self.summary_report()['coverage_and_conversations_ready'])


if __name__ == '__main__':
    unittest.main()
