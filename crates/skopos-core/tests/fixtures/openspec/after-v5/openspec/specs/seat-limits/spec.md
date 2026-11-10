# seat-limits Specification

## Purpose
What happens to seats when an org moves to a plan with fewer seats.

## Requirements

### Requirement: FR4 - A plan change that leaves more seats in use than the new... (v4)

A plan change that leaves more seats in use than the new limit is blocked.

Reference: `pa_2Xc5Rv8Tn1Mb4Kz7@4#FR4`. Priority: must

#### Scenario: Example 1

- **WHEN** 12 seats in use, new limit 10. The change is blocked.
- **THEN** the system behaves as the example says

<!-- skopos-export: 71f800399392c1a2 -->

### Requirement: FR6 - A blocked plan change names the members above the new limit

A blocked plan change names the members above the new limit.

Reference: `pa_2Xc5Rv8Tn1Mb4Kz7@5#FR6`. Priority: must

#### Scenario: Example 1

- **WHEN** 12 seats in use, new limit 10. The message names two members.
- **THEN** the system behaves as the example says

<!-- skopos-export: cff6a32ed2d787a9 -->
