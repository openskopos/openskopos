# Spec Delta

## Purpose

The requirements of the OpenSkopos document "Intent: self-serve plan downgrades" (pa_7Hk2mQ9xR4tV8wZ1@2).

## ADDED Requirements

### Requirement: FR1 - The billing page lets an org admin pick a smaller plan

The billing page lets an org admin pick a smaller plan.

Reference: `pa_7Hk2mQ9xR4tV8wZ1@2#FR1`. Priority: must

#### Scenario: Example 1

- **WHEN** An admin on Team picks Starter. The page shows Starter from 1 December.
- **THEN** the system behaves as the example says

#### Scenario: Edge case 2

- **WHEN** An admin with 12 seats picks a plan with 10 seats. The page blocks the change and names the seat limit.
- **THEN** the system behaves as the example says

#### Scenario: Must not happen 3

- **WHEN** A member who is not an admin sees the plan picker.
- **THEN** this MUST NOT happen
