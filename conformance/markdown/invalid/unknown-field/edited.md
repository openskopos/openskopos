---
schema: https://openskopos.org/schema/artifact/0.1.json
id: pa_7Hk2mQ9xR4tV8wZ1
version: 2
kind: intent
status: final
---

# Intent: self-serve plan downgrades

## Goals

- G1: Customers can lower their bill without waiting for support.
  - for: Org admins on monthly plans
  - colour: blue

## Success signals

- S1: A customer can downgrade without contacting support.
  - verification: The downgrade feature test passes in CI.
- S3: Downgrades take effect at the end of the paid period.

## Requirements

- FR1: The billing page lets an org admin pick a smaller plan.
  - priority: must
  - example: An admin on Team picks Starter. The page shows Starter from 1 December.
  - edge: An admin with 12 seats picks a plan with 10 seats. The page blocks the change and names the seat limit.
  - forbidden: A member who is not an admin sees the plan picker.

## Stories

- ST1: As an org admin, I want to pick a smaller plan, so that we pay less next month.
  - ac: The billing page shows the new plan and the date it starts.

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

## Open questions

- Q1: What happens to annual plans?
  - owner: P1
  - blocking: false
  - options: Downgrade at the end of the year
  - options: Refund the difference monthly

## Decisions

- D1: Downgrades are scheduled, never immediate.
  - rationale: Matches how the payment provider bills periods.
  - alternatives: Immediate downgrade with proration
  - consequences: Customers pay the old price until the period ends.
  - decided_by: P2

## Terms

- T1: Downgrade
  - definition: A move to a cheaper plan that starts at the end of the paid period.
  - aliases: plan change down
  - code_names: Billing.schedule_plan_change/3

## From other artifacts

### Critiques from Red team: retry failed card payments (pa_3fQ8nV2kLm7Xc1Rt@1)

- pa_3fQ8nV2kLm7Xc1Rt@1#CR1: The spec retries every failure, but the code already treats card_declined as final.
  - severity: blocker
  - category: incorrect
  - target.section: Retry policy
  - resolution.outcome: fix_spec
  - resolution.note: Retry only network errors and processor 5xx.
  - resolution.decided_by: P1
