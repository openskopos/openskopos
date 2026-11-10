# seat-limits Specification

## Purpose
What happens to seats when an org moves to a plan with fewer seats.

## Requirements

### Requirement: FR3 - The seat count never goes below zero

The seat count never goes below zero, also while a plan change runs.

Reference: `pa_2Xc5Rv8Tn1Mb4Kz7@4#FR3`. Priority: should

An invariant. A property-based test fits it.

#### Scenario: FR3 holds

- **WHEN** the change is done
- **THEN** The seat count never goes below zero, also while a plan change runs.

<!-- skopos-export: fe054795519772e4 -->

### Requirement: FR4 - A plan change that leaves more seats in use than the new... (v4)

A plan change that leaves more seats in use than the new limit is blocked.

Reference: `pa_2Xc5Rv8Tn1Mb4Kz7@4#FR4`. Priority: must

#### Scenario: Example 1

- **WHEN** 12 seats in use, new limit 10. The change is blocked.
- **THEN** the system behaves as the example says

<!-- skopos-export: 71f800399392c1a2 -->

### Requirement: FR5 - An admin can see the seat limit of each plan before a change

An admin can see the seat limit of each plan before a change.

Reference: `pa_2Xc5Rv8Tn1Mb4Kz7@4#FR5`. Priority: should

#### Scenario: Example 1

- **WHEN** The plan page shows 10 seats for the Team plan.
- **THEN** the system behaves as the example says

#### Scenario: Success signal S3

- **WHEN** Support tags tickets with seat-limit for one month.
- **THEN** Fewer support tickets ask about seat limits. `pa_2Xc5Rv8Tn1Mb4Kz7@4#S3`

<!-- skopos-export: 44b26ffb55b046b3 -->
