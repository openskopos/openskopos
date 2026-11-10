# notifications Specification

## Purpose
Tells admins about changes to their org by email, so that nothing changes without them knowing.

## Requirements

### Requirement: Plan change email
The system SHALL send an email to every admin of an org when the plan of the org changes.

#### Scenario: Downgrade
- **WHEN** an admin moves the org from Business to Team
- **THEN** every admin of the org gets an email that names both plans

#### Scenario: No admins
- **WHEN** the org has no admin with a confirmed email address
- **THEN** the system sends no email and logs a warning
