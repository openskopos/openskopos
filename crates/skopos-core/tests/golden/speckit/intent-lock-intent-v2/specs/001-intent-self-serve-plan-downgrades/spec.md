# Feature Specification: Intent: self-serve plan downgrades

**Feature Branch**: `001-intent-self-serve-plan-downgrades`

**Created**: 2026-11-05

**Status**: Draft

**Input**: OpenSkopos document `pa_7Hk2mQ9xR4tV8wZ1@2`. Each line ends with the reference string of its item.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - As an org admin, I want to pick a smaller plan, so that... (Priority: P1)

As an org admin, I want to pick a smaller plan, so that we pay less next month. `pa_7Hk2mQ9xR4tV8wZ1@2#ST1`

**Why this priority**: it refines FR1, a "must" requirement.

**Acceptance Scenarios**:

1. The billing page shows the new plan and the date it starts. `pa_7Hk2mQ9xR4tV8wZ1@2#ST1`

### Edge Cases

- An admin with 12 seats picks a plan with 10 seats. The page blocks the change and names the seat limit. `pa_7Hk2mQ9xR4tV8wZ1@2#FR1`

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The billing page lets an org admin pick a smaller plan. `pa_7Hk2mQ9xR4tV8wZ1@2#FR1`
  - Priority: must
  - Example: An admin on Team picks Starter. The page shows Starter from 1 December.
  - Must not happen: A member who is not an admin sees the plan picker.
- [NEEDS CLARIFICATION: What happens to annual plans?] `pa_7Hk2mQ9xR4tV8wZ1@2#Q1`
  - Options: Downgrade at the end of the year; Refund the difference monthly

### Key Entities

- **Downgrade**: A move to a cheaper plan that starts at the end of the paid period. `pa_7Hk2mQ9xR4tV8wZ1@2#T1`
  - Also called: plan change down
  - In the code: `Billing.schedule_plan_change/3`

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A customer can downgrade without contacting support. `pa_7Hk2mQ9xR4tV8wZ1@2#S1`
  - Verification: The downgrade feature test passes in CI.
- **SC-003**: Downgrades take effect at the end of the paid period. `pa_7Hk2mQ9xR4tV8wZ1@2#S3`

## Assumptions

- The payment provider can schedule a plan change for the end of the paid period. (confirmed) `pa_7Hk2mQ9xR4tV8wZ1@2#A1`
- Fewer than 5% of customers are on annual plans. (unconfirmed) `pa_7Hk2mQ9xR4tV8wZ1@2#A2`
- Out of scope: Refunds for unused time. Reason: Finance decides refunds case by case. `pa_7Hk2mQ9xR4tV8wZ1@2#N1`

## Not exported

These items of pa_7Hk2mQ9xR4tV8wZ1@2 are not in these files. Read them in the OpenSkopos document.

- `pa_7Hk2mQ9xR4tV8wZ1@2#R1` (risk): proposed: the team has not agreed to it yet
