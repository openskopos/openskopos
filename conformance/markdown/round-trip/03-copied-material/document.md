---
schema: https://openskopos.org/schema/artifact/0.1.json
id: pa_7Hk2mQ9xR4tV8wZ1
version: 2
kind: intent
status: draft
method:
  id: intent-lock
session:
  id: '4907'
  title: 'Intent Lock: downgrades, round 2'
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
decider: P2
parents:
  - rel: previous_version
    artifact: pa_7Hk2mQ9xR4tV8wZ1
    version: 1
  - rel: input
    artifact: pa_3fQ8nV2kLm7Xc1Rt
    version: 1
    kind: critique
  - rel: input
    source:
      kind: ticket
      title: BILL-901 Downgrade flow
      uri: https://tracker.example.com/BILL-901
id_counters:
  G: 1
  S: 3
  FR: 1
  ST: 1
  C: 2
  A: 2
  N: 1
  R: 1
  Q: 1
  D: 1
  T: 1
created_at: '2026-11-05T09:00:00Z'
---

# Intent: self-serve plan downgrades

## Goals

- G1: Customers can lower their bill without waiting for support.
  - for: Org admins on monthly plans

## Success signals

- S1: A customer can downgrade without contacting support.
  - verification: The downgrade feature test passes in CI.
  - verifies: G1
- S3: Downgrades take effect at the end of the paid period.

## Requirements

- FR1: The billing page lets an org admin pick a smaller plan.
  - priority: must
  - normal: An admin on Team picks Starter. The page shows Starter from 1 December.
  - edge: An admin with 12 seats picks a plan with 10 seats. The page blocks the change and names the seat limit.
  - forbidden: A member who is not an admin sees the plan picker.

## Stories

- ST1: As an org admin, I want to pick a smaller plan, so that we pay less next month.
  - ac: The billing page shows the new plan and the date it starts.
  - refines: FR1

## Constraints

- C1: Never change the payment webhook contract.
  - modality: never
  - rationale: The billing sync depends on it.
  - scope: lib/billing/
  - enforced_by: Payment webhook contract tests in CI
- C2: Ask before adding a database migration.
  - modality: ask_first
  - scope: priv/repo/migrations/
  - approver: Tech lead
  - bring: The migration
  - bring: A rollback plan

## Assumptions

- A1: The payment provider can schedule a plan change for the end of the paid period.
  - confirmation: confirmed
  - owner: P2
  - evidence:
    - code: lib/billing/provider.ex
      - repo: acme/billing
      - lines: 88-97
      - commit: b4c5d6e
- A2: Fewer than 5% of customers are on annual plans.
  - confirmation: unconfirmed
  - owner: P1
  - confirm_by: 2026-11-20

## Non-goals

- N1: Refunds for unused time.
  - reason: Finance decides refunds case by case.
  - deferred: false

## Risks

- R1: Seats above the new plan limit lock users out.
  - status: proposed
  - likelihood: medium
  - impact: high
  - mitigation: Block the downgrade until seats fit the plan.
  - threatens: FR1

## Open questions

- Q1: What happens to annual plans?
  - owner: P1
  - blocking: false
  - option: Downgrade at the end of the year
  - option: Refund the difference monthly

## Decisions

- D1: Downgrades are scheduled, never immediate.
  - rationale: Matches how the payment provider bills periods.
  - alternative: Immediate downgrade with proration
  - consequence: Customers pay the old price until the period ends.
  - decided_by: P2
  - resolves: pa_3fQ8nV2kLm7Xc1Rt@1#CR1

## Terms

- T1: Downgrade
  - definition: A move to a cheaper plan that starts at the end of the paid period.
  - alias: plan change down
  - code_name: Billing.schedule_plan_change/3

## From other artifacts

### Critiques from Red team: retry failed card payments (pa_3fQ8nV2kLm7Xc1Rt@1)

- pa_3fQ8nV2kLm7Xc1Rt@1#CR1: The spec retries every failure, but the code already treats card_declined as final.
  - severity: blocker
  - category: incorrect
  - target: Retry policy
  - resolution: fix_spec
    - note: Retry only network errors and processor 5xx.
    - decided_by: P1
  - evidence:
    - quote: Retry each failed charge up to three times.
      - from: Payment retry spec (draft 3)
      - location: Retry policy
    - code: lib/billing/retry.ex
      - repo: acme/billing
      - lines: 41-48
      - commit: a1b2c3d
  - proposed_by: P3
  - votes: 2 yes, 0 no, 2 eligible
