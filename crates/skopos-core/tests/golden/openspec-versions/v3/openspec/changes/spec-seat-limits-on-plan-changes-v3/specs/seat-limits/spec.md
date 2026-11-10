# Spec Delta

## ADDED Requirements

### Requirement: FR4 - A plan change that leaves more seats in use than the new...

A plan change that leaves more seats in use than the new limit is blocked.

Reference: `pa_2Xc5Rv8Tn1Mb4Kz7@3#FR4`. Priority: must

#### Scenario: Example 1

- **WHEN** 12 seats in use, new limit 10. The change is blocked.
- **THEN** the system behaves as the example says

#### Scenario: Edge case 2

- **WHEN** 10 seats in use, new limit 10. The change goes ahead.
- **THEN** the system behaves as the example says

<!-- skopos-export: a70d0b5cda46a23d -->

## REMOVED Requirements

### Requirement: FR2 - A plan change that leaves more seats in use than the new...

**Reason**: pa_2Xc5Rv8Tn1Mb4Kz7@3 replaces FR2 with FR4.
**Migration**: Follow FR4.
