---
schema: https://openskopos.org/schema/artifact/0.1.json
id: pa_2Xc5Rv8Tn1Mb4Kz7
version: 3
kind: spec
status: draft
---

# Spec: seat limits on plan changes

What happens to seats when an org moves to a plan with fewer seats.

This version adds the critique from the red team.

## Goals

- G2: No user loses access by surprise when the org changes plan.
  - for: Members of orgs that downgrade

## Success signals

- S2: No support ticket about lost access after a plan change.
  - status: proposed
  - verification: Support tags tickets with plan-change for one month.

## Requirements

- FR4: A plan change that leaves more seats in use than the new limit is blocked.
  - priority: must
  - example: 12 seats in use, new limit 10. The change is blocked.
  - edge: 10 seats in use, new limit 10. The change goes ahead.
- FR3: The seat count never goes below zero.
  - priority: should

## Stories

- ST1: As an org admin, I want to see which seats block a plan change, so that I can free them.
  - ac: The page lists the members above the limit.
  - forbidden: A member who is not an admin sees the list.

## Constraints

- C1: Always run the billing contract tests before a merge.
  - modality: always
  - enforced_by: CI job billing-contract
- C3: Prefer the existing Seat module over new tables.
  - modality: prefer
  - rationale: One owner for seat data.
  - scope: lib/billing/
- C4: Avoid new background jobs for this work.
  - modality: avoid
  - scope: this work only

## Assumptions

- A1: Seat counts are exact at the time of the plan change.
  - confirmation: refuted
  - owner: P2
- A2: Seat counts are at most one minute late.
  - owner: P2
  - confirm_by: 2026-12-01

## Non-goals

- N1: Automatic removal of members above the limit.
  - reason: An admin chooses who leaves.
  - deferred: true

## Risks

- R1: A late seat count lets a blocked change through.
  - likelihood: low
  - impact: high
  - mitigation: Recount seats inside the plan change.
  - owner: P1

## Open questions

- Q2: Do guest users count as seats?
  - owner: P1
  - blocking: true
  - answer: No, guests are free.

## Decisions

- D1: Recount seats inside the plan change.
  - rationale: The background count can be late.
  - decided_by: P1

## Terms

- T1: Seat limit
  - definition: The highest number of seats that a plan allows.
  - code_names: Plan.max_seats

## Critiques

- CR1: Version 2 trusts a seat count that a background job writes.
  - severity: major
  - category: incorrect
  - target.artifact: pa_2Xc5Rv8Tn1Mb4Kz7
  - target.version: 2
  - target.item: A1
  - resolution.outcome: fix_code
  - resolution.note: The spec is right, the recount is wrong.
  - resolution.decided_by: P1

## From other artifacts

### Terms from Billing glossary (pa_9Lp4Wd7Ks2Hq6Nb3@1)

- pa_9Lp4Wd7Ks2Hq6Nb3@1#T1: Seat
  - definition: One user who can sign in to an org.
  - aliases: licence
  - code_names: Billing.Seat
  - code_names: seats_used
- pa_9Lp4Wd7Ks2Hq6Nb3@1#T2: Plan
  - definition: The product tier that an org pays for, with a seat limit.
  - code_names: Billing.Plan
