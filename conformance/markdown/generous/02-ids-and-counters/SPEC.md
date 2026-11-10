---
kind: spec
id: pa_4Gh7Jk1Lm3Np5Qr8
id_counters:
  FR: 6
---

# Seat limits

## Requirements

- **FR2**: A plan change that leaves too many seats in use is blocked.
- [FR5] The seat count never goes below zero.
- fr7. An admin can see the seat count on the billing page.
- An admin can see the seat limit on the billing page.
- FR5: The page names the members above the limit.
- S1: The change takes effect at the end of the period.

## Risks

- R3 - A late seat count lets a blocked change through.
  - likelihood: Low
  - impact: HIGH
