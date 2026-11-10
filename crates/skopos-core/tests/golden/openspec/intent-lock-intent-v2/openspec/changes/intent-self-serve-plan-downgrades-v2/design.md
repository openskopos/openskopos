# Design: Intent: self-serve plan downgrades

## Context

Constraints:

- **Never**: Never change the payment webhook contract. `pa_7Hk2mQ9xR4tV8wZ1@2#C1`
  - Why: The billing sync depends on it.
  - Applies to: lib/billing/
  - Enforced by: Payment webhook contract tests in CI
- **Ask first**: Ask before adding a database migration. `pa_7Hk2mQ9xR4tV8wZ1@2#C2`
  - Applies to: priv/repo/migrations/
  - Ask: Tech lead
  - Bring: The migration; A rollback plan

Assumptions:

- The payment provider can schedule a plan change for the end of the paid period. (confirmed) `pa_7Hk2mQ9xR4tV8wZ1@2#A1`
- Fewer than 5% of customers are on annual plans. (unconfirmed) `pa_7Hk2mQ9xR4tV8wZ1@2#A2`
  - Confirm by: 2026-11-20

## Goals / Non-Goals

**Goals:**

- See "Why" in proposal.md.

**Non-Goals:**

- Refunds for unused time. `pa_7Hk2mQ9xR4tV8wZ1@2#N1`
  - Reason: Finance decides refunds case by case.

## Decisions

### D1: Downgrades are scheduled, never immediate

- **Decision**: Downgrades are scheduled, never immediate. `pa_7Hk2mQ9xR4tV8wZ1@2#D1`
- **Rationale**: Matches how the payment provider bills periods.
- **Alternatives considered**: Immediate downgrade with proration
- **Consequences**: Customers pay the old price until the period ends.

## Open Questions

- What happens to annual plans? `pa_7Hk2mQ9xR4tV8wZ1@2#Q1`
  - Options: Downgrade at the end of the year; Refund the difference monthly
