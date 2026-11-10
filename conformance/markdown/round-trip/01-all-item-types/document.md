---
schema: https://openskopos.org/schema/artifact/0.1.json
id: pa_2Xc5Rv8Tn1Mb4Kz7
version: 3
kind: spec
status: draft
method:
  id: elaboration-mob
  version: '2'
session:
  id: s-7781
  title: Seat limits, round 3
  url: https://example.com/sessions/7781
participants:
  - id: P1
    name: Ana
    kind: human
  - id: P2
    name: Lee
    kind: human
  - id: P3
    name: Facilitator
    kind: ai
decider: P1
parents:
  - rel: previous_version
    artifact: pa_2Xc5Rv8Tn1Mb4Kz7
    version: 2
  - rel: input
    artifact: pa_9Lp4Wd7Ks2Hq6Nb3
    version: 1
    kind: glossary
    title: Billing glossary
  - rel: input
    source:
      kind: repo_file
      title: Seat model
      repo: acme/billing
      path: lib/billing/seat.ex
      commit: 0f1e2d3c4b5a
      digest: sha256:abababababababababababababababababababababababababababababababab
  - rel: input
    source:
      kind: url
      title: Pricing page
      uri: https://example.com/pricing
id_counters:
  G: 2
  S: 2
  FR: 4
  ST: 1
  C: 4
  A: 2
  N: 1
  R: 1
  Q: 2
  D: 1
  T: 1
  CR: 1
created_at: '2026-11-08T14:00:00Z'
x-generator:
  name: example-tool
  version: 0.0.1
---

# Spec: seat limits on plan changes

What happens to seats when an org moves to a plan with fewer seats.

This version adds the critique from the red team.

## Goals

- G2: No user loses access by surprise when the org changes plan.
  - for: Members of orgs that downgrade

## Success signals

- S2: No support ticket about lost access after a plan change.
  - status: proposed
  - verification: Support tags tickets with plan-change for one month.
  - verifies: G2

## Requirements

- FR4: A plan change that leaves more seats in use than the new limit is blocked.
  - priority: must
  - example: 12 seats in use, new limit 10. The change is blocked.
  - edge: 10 seats in use, new limit 10. The change goes ahead.
  - supersedes: FR2
  - derived_from: G2

- FR3: The seat count never goes below zero.
  - priority: should

  An invariant. A property-based test fits it.

## Stories

- ST1: As an org admin, I want to see which seats block a plan change, so that I can free them.
  - ac: The page lists the members above the limit.
  - forbidden: A member who is not an admin sees the list.
  - refines: FR4

## Constraints

- C1: Always run the billing contract tests before a merge.
  - modality: always
  - enforced_by: CI job billing-contract
- C3: Prefer the existing Seat module over new tables.
  - modality: prefer
  - rationale: One owner for seat data.
  - scope: lib/billing/
- C4: Avoid new background jobs for this work.
  - modality: avoid
  - scope: this work only
  - mitigates: R1

## Assumptions

- A1: Seat counts are exact at the time of the plan change.
  - confirmation: refuted
  - owner: P2
  - evidence:
    - code: lib/billing/seat.ex
      - repo: acme/billing
      - lines: 30-44
      - commit: 0f1e2d3c4b5a
      - snippet: Task.start(fn -> recount(org) end)
- A2: Seat counts are at most one minute late.
  - owner: P2
  - confirm_by: 2026-12-01
  - contradicts: A1

## Non-goals

- N1: Automatic removal of members above the limit.
  - reason: An admin chooses who leaves.
  - deferred: true

## Risks

- R1: A late seat count lets a blocked change through.
  - likelihood: low
  - impact: high
  - mitigation: Recount seats inside the plan change.
  - owner: P1
  - threatens: FR4

## Open questions

- Q2: Do guest users count as seats?
  - owner: P1
  - blocking: true
  - answer: No, guests are free.
  - relates_to: pa_9Lp4Wd7Ks2Hq6Nb3@1#T1
- Q1: Should we email the admin?
  - status: rejected

## Decisions

- D1: Recount seats inside the plan change.
  - rationale: The background count can be late.
  - decided_by: P1
  - resolves: CR1
  - depends_on: A1

## Terms

- T1: Seat limit
  - definition: The highest number of seats that a plan allows.
  - code_name: Plan.max_seats

## Critiques

- CR1: Version 2 trusts a seat count that a background job writes.
  - severity: major
  - category: incorrect
  - target: pa_2Xc5Rv8Tn1Mb4Kz7@2#A1
  - resolution: fix_code
    - note: The spec is right, the recount is wrong.
    - decided_by: P1
  - evidence:
    - quote: Seat counts are exact.
      - from: pa_2Xc5Rv8Tn1Mb4Kz7@2#A1
    - discussion: We saw a five minute lag last week.
      - speaker: P2
    - link: https://example.com/incidents/42
      - title: Incident 42
  - proposed_by: P3
  - votes: 2 yes, 0 no, 2 eligible
  - x-tracker: BILL-77
