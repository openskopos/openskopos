# Requirements Document

## Introduction

Intent: self-serve plan downgrades. Exported from the OpenSkopos document `pa_7Hk2mQ9xR4tV8wZ1@2`. Each line ends with the reference string of its item.

Goals:

- Customers can lower their bill without waiting for support. `pa_7Hk2mQ9xR4tV8wZ1@2#G1`
  - For: Org admins on monthly plans

Success signals:

- A customer can downgrade without contacting support. `pa_7Hk2mQ9xR4tV8wZ1@2#S1`
  - Verification: The downgrade feature test passes in CI.
- Downgrades take effect at the end of the paid period. `pa_7Hk2mQ9xR4tV8wZ1@2#S3`

## Glossary

- **Downgrade**: A move to a cheaper plan that starts at the end of the paid period. `pa_7Hk2mQ9xR4tV8wZ1@2#T1`
  - Also called: plan change down
  - In the code: `Billing.schedule_plan_change/3`

## Requirements

### Requirement 1: The billing page lets an org admin pick a smaller plan

**User Story:** As an org admin, I want to pick a smaller plan, so that we pay less next month. `pa_7Hk2mQ9xR4tV8wZ1@2#ST1`

#### Acceptance Criteria

1. The billing page lets an org admin pick a smaller plan. `pa_7Hk2mQ9xR4tV8wZ1@2#FR1` (priority: must)
2. Example: An admin on Team picks Starter. The page shows Starter from 1 December. `pa_7Hk2mQ9xR4tV8wZ1@2#FR1`
3. Edge case: An admin with 12 seats picks a plan with 10 seats. The page blocks the change and names the seat limit. `pa_7Hk2mQ9xR4tV8wZ1@2#FR1`
4. Must not happen: A member who is not an admin sees the plan picker. `pa_7Hk2mQ9xR4tV8wZ1@2#FR1`
5. The billing page shows the new plan and the date it starts. `pa_7Hk2mQ9xR4tV8wZ1@2#ST1`

## Not exported

These items of pa_7Hk2mQ9xR4tV8wZ1@2 are not in these files. Read them in the OpenSkopos document.

- `pa_7Hk2mQ9xR4tV8wZ1@2#A1` (assumption): Kiro has no place for assumptions
- `pa_7Hk2mQ9xR4tV8wZ1@2#A2` (assumption): Kiro has no place for assumptions
- `pa_7Hk2mQ9xR4tV8wZ1@2#N1` (non_goal): Kiro has no place for non-goals
- `pa_7Hk2mQ9xR4tV8wZ1@2#R1` (risk): proposed: the team has not agreed to it yet
- `pa_7Hk2mQ9xR4tV8wZ1@2#Q1` (open_question): Kiro has no place for open questions
