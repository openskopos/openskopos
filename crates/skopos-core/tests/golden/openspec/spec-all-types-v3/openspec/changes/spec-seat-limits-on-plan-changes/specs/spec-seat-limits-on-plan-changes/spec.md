# Spec Delta

## Purpose

What happens to seats when an org moves to a plan with fewer seats.

## ADDED Requirements

### Requirement: FR4 - A plan change that leaves more seats in use than the new...

A plan change that leaves more seats in use than the new limit is blocked.

Reference: `pa_2Xc5Rv8Tn1Mb4Kz7@3#FR4`. Priority: must

#### Scenario: Example 1

- **WHEN** 12 seats in use, new limit 10. The change is blocked.
- **THEN** the system behaves as the example says

#### Scenario: Edge case 2

- **WHEN** 10 seats in use, new limit 10. The change goes ahead.
- **THEN** the system behaves as the example says

### Requirement: FR3 - The seat count never goes below zero

The seat count never goes below zero.

Reference: `pa_2Xc5Rv8Tn1Mb4Kz7@3#FR3`. Priority: should

An invariant. A property-based test fits it.

#### Scenario: FR3 holds

- **WHEN** the change is done
- **THEN** The seat count never goes below zero.
