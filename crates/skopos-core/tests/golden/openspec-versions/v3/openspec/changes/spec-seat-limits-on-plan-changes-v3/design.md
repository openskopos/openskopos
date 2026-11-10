# Design: Spec: seat limits on plan changes

## Context

Constraints:

- **Always**: Always run the billing contract tests before a merge. `pa_2Xc5Rv8Tn1Mb4Kz7@3#C1`
  - Enforced by: CI job billing-contract
- **Prefer**: Prefer the existing Seat module over new tables. `pa_2Xc5Rv8Tn1Mb4Kz7@3#C3`
  - Why: One owner for seat data.
  - Applies to: lib/billing/
- **Avoid**: Avoid new background jobs for this work. `pa_2Xc5Rv8Tn1Mb4Kz7@3#C4`
  - Applies to: this work only

Assumptions:

- Seat counts are exact at the time of the plan change. (refuted) `pa_2Xc5Rv8Tn1Mb4Kz7@3#A1`
- Seat counts are at most one minute late. (unconfirmed) `pa_2Xc5Rv8Tn1Mb4Kz7@3#A2`
  - Confirm by: 2026-12-01

## Goals / Non-Goals

**Goals:**

- See "Why" in proposal.md.

**Non-Goals:**

- Automatic removal of members above the limit. `pa_2Xc5Rv8Tn1Mb4Kz7@3#N1`
  - Reason: An admin chooses who leaves.
  - Deferred: later, not never.

## Decisions

### D1: Recount seats inside the plan change

- **Decision**: Recount seats inside the plan change. `pa_2Xc5Rv8Tn1Mb4Kz7@3#D1`
- **Rationale**: The background count can be late.

## Risks / Trade-offs

- [Risk] A late seat count lets a blocked change through. → Recount seats inside the plan change. `pa_2Xc5Rv8Tn1Mb4Kz7@3#R1`
  - likelihood low, impact high

## Open Questions

- Do guest users count as seats? `pa_2Xc5Rv8Tn1Mb4Kz7@3#Q2`
  - Blocking: the work cannot continue without an answer.
  - Answer: No, guests are free.
