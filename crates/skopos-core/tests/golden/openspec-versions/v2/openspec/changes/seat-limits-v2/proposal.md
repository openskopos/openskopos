# Proposal: Spec: seat limits on plan changes

Exported from the OpenSkopos document `pa_2Xc5Rv8Tn1Mb4Kz7@2`. Each item ends with its reference string.

## Why

What happens to seats when an org moves to a plan with fewer seats.

Goals:

- No user loses access by surprise when the org changes plan. `pa_2Xc5Rv8Tn1Mb4Kz7@2#G2`
  - For: Members of orgs that downgrade

Success signals:

- No support ticket about lost access after a plan change. `pa_2Xc5Rv8Tn1Mb4Kz7@2#S2`
  - Verification: Support tags tickets with plan-change for one month.

## What Changes

- `seat-limits`: adds FR2, FR3.
- Story: As an org admin, I want to see which seats block a plan change, so that I can free them. `pa_2Xc5Rv8Tn1Mb4Kz7@2#ST1`
  - Refines: FR2
  - Acceptance: The page lists the members above the limit.
  - Must not happen: A member who is not an admin sees the list.

## Capabilities

### New Capabilities

- `seat-limits`: Spec: seat limits on plan changes

## Impact

- Source: OpenSkopos `pa_2Xc5Rv8Tn1Mb4Kz7@2`. A reference string `<artifact>@<version>#<item>` names the item that a line comes from.

## Not exported

These items of pa_2Xc5Rv8Tn1Mb4Kz7@2 are not in these files. Read them in the OpenSkopos document.

- `pa_2Xc5Rv8Tn1Mb4Kz7@2#Q1` (open_question): proposed: the team has not agreed to it yet
- `pa_2Xc5Rv8Tn1Mb4Kz7@2#T1` (term): OpenSpec has no place for terms
