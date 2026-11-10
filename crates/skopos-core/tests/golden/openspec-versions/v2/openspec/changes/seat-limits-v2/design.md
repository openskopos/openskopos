# Design: Spec: seat limits on plan changes

## Context

Constraints:

- **Always**: Always run the billing contract tests before a merge. `pa_2Xc5Rv8Tn1Mb4Kz7@2#C1`
  - Enforced by: CI job billing-contract
- **Prefer**: Prefer the existing Seat module over new tables. `pa_2Xc5Rv8Tn1Mb4Kz7@2#C3`
  - Why: One owner for seat data.
  - Applies to: lib/billing/

Assumptions:

- Seat counts are exact. (unconfirmed) `pa_2Xc5Rv8Tn1Mb4Kz7@2#A1`

## Goals / Non-Goals

**Goals:**

- See "Why" in proposal.md.

**Non-Goals:**

- Automatic removal of members above the limit. `pa_2Xc5Rv8Tn1Mb4Kz7@2#N1`
  - Reason: An admin chooses who leaves.
  - Deferred: later, not never.

## Open Questions

- Do guest users count as seats? `pa_2Xc5Rv8Tn1Mb4Kz7@2#Q2`
  - Blocking: the work cannot continue without an answer.
