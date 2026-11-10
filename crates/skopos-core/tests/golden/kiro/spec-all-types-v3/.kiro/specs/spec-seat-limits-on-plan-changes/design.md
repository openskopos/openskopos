# Design Document

## Overview

The constraints and decisions of the OpenSkopos document `pa_2Xc5Rv8Tn1Mb4Kz7@3`. The requirements are in requirements.md.

## Constraints

- **Always**: Always run the billing contract tests before a merge. `pa_2Xc5Rv8Tn1Mb4Kz7@3#C1`
  - Enforced by: CI job billing-contract
- **Prefer**: Prefer the existing Seat module over new tables. `pa_2Xc5Rv8Tn1Mb4Kz7@3#C3`
  - Why: One owner for seat data.
  - Applies to: lib/billing/
- **Avoid**: Avoid new background jobs for this work. `pa_2Xc5Rv8Tn1Mb4Kz7@3#C4`
  - Applies to: this work only

## Decisions

### D1: Recount seats inside the plan change

- **Decision**: Recount seats inside the plan change. `pa_2Xc5Rv8Tn1Mb4Kz7@3#D1`
- **Rationale**: The background count can be late.
