# Implementation Plan: Spec: seat limits on plan changes

**Branch**: `001-spec-seat-limits-on-plan-changes` | **Date**: 2026-11-08 | **Spec**: [spec.md](spec.md)

## Summary

- No user loses access by surprise when the org changes plan. `pa_2Xc5Rv8Tn1Mb4Kz7@3#G2`
  - For: Members of orgs that downgrade

## Technical Context

**Constraints**:

- **Always**: Always run the billing contract tests before a merge. `pa_2Xc5Rv8Tn1Mb4Kz7@3#C1`
  - Enforced by: CI job billing-contract
- **Prefer**: Prefer the existing Seat module over new tables. `pa_2Xc5Rv8Tn1Mb4Kz7@3#C3`
  - Why: One owner for seat data.
  - Applies to: lib/billing/
- **Avoid**: Avoid new background jobs for this work. `pa_2Xc5Rv8Tn1Mb4Kz7@3#C4`
  - Applies to: this work only
