---
kind: critique
participants:
  - id: P1
    name: Ana
    kind: human
  - id: P2
    name: Sam
    kind: human
---

# Red team: payment retries

## Critiques

- CR1: The spec retries every failure, but the code treats card_declined as final.
  - severity: Blocker
  - category: incorrect
  - target: Retry policy
  - resolution: fix spec
    - note: Retry only network errors.
    - decided by: P1
  - evidence:
    - quote: Retry each failed charge up to three times.
      - from: Payment retry spec
      - location: Retry policy
    - code: lib/billing/retry.ex:41-48
      - repo: acme/billing
      - commit: a1b2c3d
  - votes:
    - yes: 2
    - no: 0
    - eligible: 2
- CR2: The spec does not say who is told when the last retry fails.
  - severity: major
  - category: gap
  - target: pa_7Hk2mQ9xR4tV8wZ1@2#FR1
  - evidence:
    - said: Support finds out from the customer today.
      - speaker: P2
    - link: https://example.com/incidents/7
  - relates to: CR1

  Nobody owns the notification today. This needs a decision
  before the retry work starts.
