# Spec Delta

## Purpose

What happens to seats when an org moves to a plan with fewer seats.

## ADDED Requirements

### Requirement: FR2 - A plan change that leaves more seats in use than the new...

A plan change that leaves more seats in use than the new limit shows a warning.

Reference: `pa_2Xc5Rv8Tn1Mb4Kz7@2#FR2`. Priority: must

#### Scenario: FR2 holds

- **WHEN** the change is done
- **THEN** A plan change that leaves more seats in use than the new limit shows a warning.

<!-- skopos-export: d906f9a13dab74aa -->

### Requirement: FR3 - The seat count never goes below zero

The seat count never goes below zero.

Reference: `pa_2Xc5Rv8Tn1Mb4Kz7@2#FR3`. Priority: should

An invariant. A property-based test fits it.

#### Scenario: FR3 holds

- **WHEN** the change is done
- **THEN** The seat count never goes below zero.

<!-- skopos-export: fe62ffa13f843460 -->
