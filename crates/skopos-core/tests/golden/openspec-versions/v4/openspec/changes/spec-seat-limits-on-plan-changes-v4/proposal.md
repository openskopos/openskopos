# Proposal: Spec: seat limits on plan changes

Exported from the OpenSkopos document `pa_2Xc5Rv8Tn1Mb4Kz7@4`. Each item ends with its reference string.

## Why

What happens to seats when an org moves to a plan with fewer seats.

This version shows the limits before a change.

Goals:

- No user loses access by surprise when the org changes plan. `pa_2Xc5Rv8Tn1Mb4Kz7@4#G2`
  - For: Members of orgs that downgrade

## What Changes

- `seat-limits`: adds FR4, FR5; changes FR3; removes FR4.
- Story: As an org admin, I want to see which seats block a plan change, so that I can free them. `pa_2Xc5Rv8Tn1Mb4Kz7@4#ST1`
  - Refines: FR4
  - Acceptance: The page lists the members above the limit.
  - Must not happen: A member who is not an admin sees the list.

## Capabilities

### Modified Capabilities

- `seat-limits`: requirements from pa_2Xc5Rv8Tn1Mb4Kz7@4

## Impact

- Source: OpenSkopos `pa_2Xc5Rv8Tn1Mb4Kz7@4`. A reference string `<artifact>@<version>#<item>` names the item that a line comes from.

## Not exported

These items of pa_2Xc5Rv8Tn1Mb4Kz7@4 are not in these files. Read them in the OpenSkopos document.

- `pa_2Xc5Rv8Tn1Mb4Kz7@4#S2` (success_signal): proposed: the team has not agreed to it yet
- `pa_2Xc5Rv8Tn1Mb4Kz7@4#T1` (term): OpenSpec has no place for terms
- `pa_2Xc5Rv8Tn1Mb4Kz7@4#CR1` (critique): critiques are not exported: they are about an input, not about what to build
- `pa_2Xc5Rv8Tn1Mb4Kz7@4#Q1` (open_question): rejected: an agent must not act on it
