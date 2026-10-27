#!/usr/bin/env python3
"""Read-only CodeRabbit coverage diagnostics; never authorizes or performs a merge.

Formal review evidence must match the current head and known publication format.
A status/check badge alone cannot establish coverage. Unknown/incomplete evidence
fails closed. This does not enforce owner identity separation or validate code.
"""
import argparse
import json
import re
import subprocess

BOT = 'coderabbitai[bot]'
PUBLICATION = re.compile(r'<!-- coderabbit-review-publication v1 publication=([^\s]+) attempt=([^\s]+) batch=(\d+)/(\d+) -->')
THREAD_QUERY = """query($owner:String!,$name:String!,$number:Int!){
 repository(owner:$owner,name:$name){pullRequest(number:$number){
 headRefOid reviewThreads(first:100){pageInfo{hasNextPage} nodes{
 isResolved isOutdated comments(first:100){pageInfo{hasNextPage} nodes{author{login}}}
 }}}}}"""


def assess(before, after, reviews, threads):
    head = before['headRefOid']
    issues = []
    if head != after['headRefOid']:
        issues.append('head_changed_during_collection')
    if before['state'] != 'OPEN' or after['state'] != 'OPEN':
        issues.append('pull_request_not_open')
    if before['isDraft'] or after['isDraft']:
        issues.append('draft_pull_request')
    current = [review for review in reviews
               if (review.get('user') or {}).get('login') == BOT
               and review.get('commit_id') == head
               and review.get('state') in {'COMMENTED', 'APPROVED', 'CHANGES_REQUESTED'}
               and review.get('submitted_at')]
    batches = {}
    for review in current:
        marker = PUBLICATION.search(review.get('body') or '')
        if marker:
            publication, attempt, index, total = marker.groups()
            index, total = int(index), int(total)
            if 1 <= index <= total <= 1000:
                group = batches.setdefault((publication, attempt), {'totals': set(), 'parts': {}})
                group['totals'].add(total)
                group['parts'].setdefault(index, []).append(review)
    formal = []
    for group in batches.values():
        if len(group['totals']) != 1:
            continue
        total = next(iter(group['totals']))
        parts = group['parts']
        if set(parts) == set(range(1, total+1)):
            for values in parts.values():
                for review in values:
                    formal.append({'id': review['id'], 'state': review['state'],
                                   'submitted_at': review['submitted_at']})
    if not formal:
        issues.append('no_known_completed_coderabbit_review_of_current_head')
    if threads['headRefOid'] != head:
        issues.append('thread_snapshot_head_mismatch')
    connection = threads['reviewThreads']
    if connection['pageInfo']['hasNextPage']:
        issues.append('thread_list_incomplete')
    unresolved = 0
    for thread in connection['nodes']:
        comments = thread['comments']
        if comments['pageInfo']['hasNextPage']:
            issues.append('thread_comments_incomplete')
        # Any unresolved conversation blocks readiness, including outdated ones.
        # Do not infer that a developer reply independently closed a finding.
        if not thread['isResolved']:
            unresolved += 1
    if unresolved:
        issues.append('unresolved_review_threads')
    if any(review['state'] == 'CHANGES_REQUESTED' for review in current):
        # Conservative: do not infer dismissal/supersession from a later comment.
        issues.append('current_head_changes_requested')
    return {'schema_version': 1, 'head': head,
            'known_current_head_reviews': formal, 'unresolved_threads': unresolved,
            'coverage_and_conversations_ready': not issues,
            'blockers': sorted(set(issues)), 'merge_authorized': False,
            'scope': 'review coverage/conversations only; not CI, code correctness, owner approval or merge authority'}


def gh_json(*args):
    # Fixed argv and check=True; never execute repository/review-provided text.
    result = subprocess.run(['gh', *args], check=True, capture_output=True, text=True)
    return json.loads(result.stdout)


def collect(repository, number):
    owner, name = repository.split('/')
    if not all(re.fullmatch(r'[A-Za-z0-9_.-]+', part) for part in (owner, name)):
        raise ValueError('Expected OWNER/REPOSITORY')
    fields = 'headRefOid,state,isDraft'
    before = gh_json('pr', 'view', str(number), '--repo', repository, '--json', fields)
    pages = gh_json('api', '--paginate', '--slurp',
                    f'repos/{owner}/{name}/pulls/{number}/reviews')
    reviews = [review for page in pages for review in page]
    response = gh_json('api', 'graphql', '-f', 'query='+THREAD_QUERY,
                       '-f', 'owner='+owner, '-f', 'name='+name, '-F', 'number='+str(number))
    if response.get('errors'):
        raise ValueError('GraphQL review evidence unavailable')
    threads = response['data']['repository']['pullRequest']
    after = gh_json('pr', 'view', str(number), '--repo', repository, '--json', fields)
    return assess(before, after, reviews, threads)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('repository', help='OWNER/REPOSITORY')
    parser.add_argument('pull_request', type=int)
    args = parser.parse_args()
    try:
        if args.pull_request <= 0:
            raise ValueError('Pull request number must be positive')
        report = collect(args.repository, args.pull_request)
    except (ValueError, KeyError, TypeError, AttributeError, OSError, subprocess.CalledProcessError) as error:
        # Do not echo raw API bodies/authentication stderr into public logs.
        report = {'schema_version': 1, 'coverage_and_conversations_ready': False,
                  'merge_authorized': False, 'blockers': ['review_evidence_unavailable'],
                  'error_type': type(error).__name__}
    print(json.dumps(report, indent=2))
    return 0 if report['coverage_and_conversations_ready'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
