# Feature Specification: Spec: seat limits on plan changes

**Feature Branch**: `001-spec-seat-limits-on-plan-changes`

**Created**: 2026-11-08

**Status**: Draft

**Input**: OpenSkopos document `pa_2Xc5Rv8Tn1Mb4Kz7@3`. Each line ends with the reference string of its item.

What happens to seats when an org moves to a plan with fewer seats.

This version adds the critique from the red team.

## Clarifications

### Session 2026-11-08

- Q: Do guest users count as seats? → A: No, guests are free. `pa_2Xc5Rv8Tn1Mb4Kz7@3#Q2`

## User Scenarios & Testing *(mandatory)*

### User Story 1 - As an org admin, I want to see which seats block a plan... (Priority: P1)

As an org admin, I want to see which seats block a plan change, so that I can free them. `pa_2Xc5Rv8Tn1Mb4Kz7@3#ST1`

**Why this priority**: it refines FR4, a "must" requirement.

**Acceptance Scenarios**:

1. The page lists the members above the limit. `pa_2Xc5Rv8Tn1Mb4Kz7@3#ST1`
2. Must not happen: A member who is not an admin sees the list. `pa_2Xc5Rv8Tn1Mb4Kz7@3#ST1`

### Edge Cases

- 10 seats in use, new limit 10. The change goes ahead. `pa_2Xc5Rv8Tn1Mb4Kz7@3#FR4`

## Requirements *(mandatory)*

### Functional Requirements

- **FR-004**: A plan change that leaves more seats in use than the new limit is blocked. `pa_2Xc5Rv8Tn1Mb4Kz7@3#FR4`
  - Priority: must
  - Example: 12 seats in use, new limit 10. The change is blocked.
- **FR-003**: The seat count never goes below zero. `pa_2Xc5Rv8Tn1Mb4Kz7@3#FR3`
  - Priority: should

  An invariant. A property-based test fits it.

### Key Entities

- **Seat limit**: The highest number of seats that a plan allows. `pa_2Xc5Rv8Tn1Mb4Kz7@3#T1`
  - In the code: `Plan.max_seats`

## Assumptions

- Seat counts are exact at the time of the plan change. (refuted) `pa_2Xc5Rv8Tn1Mb4Kz7@3#A1`
- Seat counts are at most one minute late. (unconfirmed) `pa_2Xc5Rv8Tn1Mb4Kz7@3#A2`
- Out of scope: Automatic removal of members above the limit. Reason: An admin chooses who leaves. (deferred: later, not never) `pa_2Xc5Rv8Tn1Mb4Kz7@3#N1`

## Not exported

These items of pa_2Xc5Rv8Tn1Mb4Kz7@3 are not in these files. Read them in the OpenSkopos document.

- `pa_2Xc5Rv8Tn1Mb4Kz7@3#S2` (success_signal): proposed: the team has not agreed to it yet
- `pa_2Xc5Rv8Tn1Mb4Kz7@3#R1` (risk): Spec Kit has no place for risks
- `pa_2Xc5Rv8Tn1Mb4Kz7@3#CR1` (critique): critiques are not exported: they are about an input, not about what to build
- `pa_2Xc5Rv8Tn1Mb4Kz7@3#Q1` (open_question): rejected: an agent must not act on it
