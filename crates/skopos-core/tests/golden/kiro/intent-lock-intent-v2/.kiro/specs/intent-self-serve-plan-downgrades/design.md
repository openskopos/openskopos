# Design Document

## Overview

The constraints and decisions of the OpenSkopos document `pa_7Hk2mQ9xR4tV8wZ1@2`. The requirements are in requirements.md.

## Constraints

- **Never**: Never change the payment webhook contract. `pa_7Hk2mQ9xR4tV8wZ1@2#C1`
  - Why: The billing sync depends on it.
  - Applies to: lib/billing/
  - Enforced by: Payment webhook contract tests in CI
- **Ask first**: Ask before adding a database migration. `pa_7Hk2mQ9xR4tV8wZ1@2#C2`
  - Applies to: priv/repo/migrations/
  - Ask: Tech lead
  - Bring: The migration; A rollback plan

## Decisions

### D1: Downgrades are scheduled, never immediate

- **Decision**: Downgrades are scheduled, never immediate. `pa_7Hk2mQ9xR4tV8wZ1@2#D1`
- **Rationale**: Matches how the payment provider bills periods.
- **Alternatives considered**: Immediate downgrade with proration
- **Consequences**: Customers pay the old price until the period ends.
