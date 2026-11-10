# Implementation Plan: Intent: self-serve plan downgrades

**Branch**: `001-intent-self-serve-plan-downgrades` | **Date**: 2026-11-05 | **Spec**: [spec.md](spec.md)

## Summary

- Customers can lower their bill without waiting for support. `pa_7Hk2mQ9xR4tV8wZ1@2#G1`
  - For: Org admins on monthly plans

## Technical Context

**Constraints**:

- **Never**: Never change the payment webhook contract. `pa_7Hk2mQ9xR4tV8wZ1@2#C1`
  - Why: The billing sync depends on it.
  - Applies to: lib/billing/
  - Enforced by: Payment webhook contract tests in CI
- **Ask first**: Ask before adding a database migration. `pa_7Hk2mQ9xR4tV8wZ1@2#C2`
  - Applies to: priv/repo/migrations/
  - Ask: Tech lead
  - Bring: The migration; A rollback plan
