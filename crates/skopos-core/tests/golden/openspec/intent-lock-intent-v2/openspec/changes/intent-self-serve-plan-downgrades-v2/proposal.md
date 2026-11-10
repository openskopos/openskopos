# Proposal: Intent: self-serve plan downgrades

Exported from the OpenSkopos document `pa_7Hk2mQ9xR4tV8wZ1@2`. Each item ends with its reference string.

## Why

Goals:

- Customers can lower their bill without waiting for support. `pa_7Hk2mQ9xR4tV8wZ1@2#G1`
  - For: Org admins on monthly plans

Success signals:

- A customer can downgrade without contacting support. `pa_7Hk2mQ9xR4tV8wZ1@2#S1`
  - Verification: The downgrade feature test passes in CI.
- Downgrades take effect at the end of the paid period. `pa_7Hk2mQ9xR4tV8wZ1@2#S3`

## What Changes

- `intent-self-serve-plan-downgrades`: adds FR1.
- Story: As an org admin, I want to pick a smaller plan, so that we pay less next month. `pa_7Hk2mQ9xR4tV8wZ1@2#ST1`
  - Refines: FR1
  - Acceptance: The billing page shows the new plan and the date it starts.

## Capabilities

### New Capabilities

- `intent-self-serve-plan-downgrades`: Intent: self-serve plan downgrades

## Impact

- Source: OpenSkopos `pa_7Hk2mQ9xR4tV8wZ1@2`. A reference string `<artifact>@<version>#<item>` names the item that a line comes from.

## Not exported

These items of pa_7Hk2mQ9xR4tV8wZ1@2 are not in these files. Read them in the OpenSkopos document.

- `pa_7Hk2mQ9xR4tV8wZ1@2#R1` (risk): proposed: the team has not agreed to it yet
- `pa_7Hk2mQ9xR4tV8wZ1@2#T1` (term): OpenSpec has no place for terms
