# Requirements Document

## Introduction

Spec: seat limits on plan changes. Exported from the OpenSkopos document `pa_2Xc5Rv8Tn1Mb4Kz7@3`. Each line ends with the reference string of its item.

What happens to seats when an org moves to a plan with fewer seats.

This version adds the critique from the red team.

Goals:

- No user loses access by surprise when the org changes plan. `pa_2Xc5Rv8Tn1Mb4Kz7@3#G2`
  - For: Members of orgs that downgrade

## Glossary

- **Seat limit**: The highest number of seats that a plan allows. `pa_2Xc5Rv8Tn1Mb4Kz7@3#T1`
  - In the code: `Plan.max_seats`

## Requirements

### Requirement 1: A plan change that leaves more seats in use than the new...

**User Story:** As an org admin, I want to see which seats block a plan change, so that I can free them. `pa_2Xc5Rv8Tn1Mb4Kz7@3#ST1`

#### Acceptance Criteria

1. A plan change that leaves more seats in use than the new limit is blocked. `pa_2Xc5Rv8Tn1Mb4Kz7@3#FR4` (priority: must)
2. Example: 12 seats in use, new limit 10. The change is blocked. `pa_2Xc5Rv8Tn1Mb4Kz7@3#FR4`
3. Edge case: 10 seats in use, new limit 10. The change goes ahead. `pa_2Xc5Rv8Tn1Mb4Kz7@3#FR4`
4. The page lists the members above the limit. `pa_2Xc5Rv8Tn1Mb4Kz7@3#ST1`
5. Must not happen: A member who is not an admin sees the list. `pa_2Xc5Rv8Tn1Mb4Kz7@3#ST1`

### Requirement 2: The seat count never goes below zero

#### Acceptance Criteria

1. The seat count never goes below zero. `pa_2Xc5Rv8Tn1Mb4Kz7@3#FR3` (priority: should)

An invariant. A property-based test fits it.

## Not exported

These items of pa_2Xc5Rv8Tn1Mb4Kz7@3 are not in these files. Read them in the OpenSkopos document.

- `pa_2Xc5Rv8Tn1Mb4Kz7@3#S2` (success_signal): proposed: the team has not agreed to it yet
- `pa_2Xc5Rv8Tn1Mb4Kz7@3#A1` (assumption): Kiro has no place for assumptions
- `pa_2Xc5Rv8Tn1Mb4Kz7@3#A2` (assumption): Kiro has no place for assumptions
- `pa_2Xc5Rv8Tn1Mb4Kz7@3#N1` (non_goal): Kiro has no place for non-goals
- `pa_2Xc5Rv8Tn1Mb4Kz7@3#R1` (risk): Kiro has no place for risks
- `pa_2Xc5Rv8Tn1Mb4Kz7@3#Q2` (open_question): Kiro has no place for open questions
- `pa_2Xc5Rv8Tn1Mb4Kz7@3#CR1` (critique): critiques are not exported: they are about an input, not about what to build
- `pa_2Xc5Rv8Tn1Mb4Kz7@3#Q1` (open_question): rejected: an agent must not act on it
