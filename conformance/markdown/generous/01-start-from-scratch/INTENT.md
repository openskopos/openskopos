# Self-serve plan downgrades

Customers want to pay less without talking to us.

## Goal

* Customers can lower their bill
  without waiting for support.
  * For: Org admins on monthly plans

## Success Signals

* A customer can downgrade without contacting support.
  * Verification: The downgrade feature test passes in CI.

## Requirements

1. The billing page lets an org admin pick a smaller plan.
   - Priority: Must
   - Examples:
     - An admin on Team picks Starter.
     - edge: 12 seats in use, plan allows 10. The change is blocked.
2. Downgrades start at the end of the paid period.

## Constraints

- Never change the payment webhook contract.
  - Modality: never
  - Enforced by: Webhook contract tests in CI
- Ask before adding a database migration.
  - modality: Ask first
  - approver: Tech lead
  - Bring:
    - The migration
    - A rollback plan

## Out of scope

- Refunds for unused time.
  - Deferred: yes

## Questions

- What happens to annual plans?
  - Blocking: no
  - Option: Downgrade at the end of the year
  - Option: Refund the difference monthly

## Glossary

- Downgrade
  - Definition: A move to a cheaper plan that starts at the end of the paid period.
  - Also called: plan change down
