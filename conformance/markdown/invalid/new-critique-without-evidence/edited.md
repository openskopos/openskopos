---
schema: https://openskopos.org/schema/artifact/0.1.json
id: pa_3fQ8nV2kLm7Xc1Rt
version: 1
kind: critique
status: final
---

# Red team: retry failed card payments

Spec Red Team on the draft retry spec, checked against the billing code.

## Critiques

- new: Retries have no upper time limit.
  - severity: minor
  - category: gap
- CR1: The spec retries every failure, but the code already treats card_declined as final.
  - severity: blocker
  - category: incorrect
  - target.section: Retry policy
  - resolution.outcome: fix_spec
  - resolution.note: Retry only network errors and processor 5xx.
  - resolution.decided_by: P1
- CR2: The spec does not say who is told when the last retry fails.
  - severity: major
  - category: gap
  - suggestion: Name who is told and how.
  - target.section: Notifications
  - resolution.outcome: defer
  - resolution.note: Separate ticket.
