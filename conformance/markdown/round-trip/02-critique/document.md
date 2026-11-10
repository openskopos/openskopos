---
schema: https://openskopos.org/schema/artifact/0.1.json
id: pa_3fQ8nV2kLm7Xc1Rt
version: 1
kind: critique
status: final
method:
  id: spec-red-team
  version: '1'
session:
  id: '4812'
  title: 'Red team: payment retries'
  url: https://example.com/sessions/4812
  ended_at: '2026-11-04T11:42:00Z'
participants:
  - id: P1
    name: Ana
    kind: human
  - id: P2
    name: Sam
    kind: human
  - id: P3
    name: Facilitator
    kind: ai
decider: P1
parents:
  - rel: input
    source:
      kind: pasted
      title: Payment retry spec (draft 3)
      digest: sha256:9c1185a5c5e9fc54612808977ee8f548b2258d31f0b1a4a0e1d4a6e4f4c3b2a1
id_counters:
  CR: 3
created_at: '2026-11-04T10:30:00Z'
finalized_at: '2026-11-04T11:40:00Z'
---

# Red team: retry failed card payments

Spec Red Team on the draft retry spec, checked against the billing code.

## Critiques

- CR1: The spec retries every failure, but the code already treats card_declined as final.
  - severity: blocker
  - category: incorrect
  - target: Retry policy
  - resolution: fix_spec
    - note: Retry only network errors and processor 5xx.
    - decided_by: P1
  - evidence:
    - quote: Retry each failed charge up to three times.
      - from: Payment retry spec (draft 3)
      - location: Retry policy
    - code: lib/billing/retry.ex
      - repo: acme/billing
      - lines: 41-48
      - commit: a1b2c3d
  - proposed_by: P3
  - votes: 2 yes, 0 no, 2 eligible
- CR2: The spec does not say who is told when the last retry fails.
  - severity: major
  - category: gap
  - suggestion: Name who is told and how.
  - target: Notifications
  - resolution: defer
    - note: Separate ticket.
  - evidence:
    - discussion: Support finds out from the customer today.
      - speaker: P2

- CR3: Three retries can double-charge if the processor times out after capture.
  - status: rejected
  - severity: major
  - category: unstated_assumption
  - suggestion: Cap retries at one until the processor confirms idempotency.
  - evidence:
    - code: lib/billing/charge.ex
      - lines: 12
      - snippet: idempotency_key: charge.id

  Rejected: charges already send an idempotency key.
