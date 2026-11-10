#!/bin/sh
# Runs exports through the real OpenSpec CLI.
#
# In a new OpenSpec project, the script exports versions 2 to 5 of one
# artifact. For each version it runs `skopos export`, `openspec validate` and
# `openspec archive`. Then it compares:
#
# - the change folder that skopos wrote with the golden files in
#   crates/skopos-core/tests/golden/openspec-versions/v<n>/, and
# - the main specs that OpenSpec wrote with the fixtures in
#   crates/skopos-core/tests/fixtures/openspec/after-v<n>/.
#
# The golden tests of skopos-core read the same fixtures, so they test the
# exporter against specs that OpenSpec itself wrote.
#
# With UPDATE_FIXTURES=1, the script writes the golden files and fixtures
# instead of comparing them.
#
# Needs `skopos` and `openspec` on the PATH. Run it from the root of the
# repository. CI pins the OpenSpec version.
set -eu

repo=$(pwd)
core="$repo/crates/skopos-core/tests"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# Copies a folder to an expected folder, or compares the two.
check_tree() {
  actual=$1
  expected=$2
  if [ "${UPDATE_FIXTURES:-}" = 1 ]; then
    rm -rf "$expected"
    mkdir -p "$(dirname "$expected")"
    cp -R "$actual" "$expected"
  elif ! diff -r "$expected" "$actual"; then
    echo "error: $actual is different from $expected" >&2
    echo "Run with UPDATE_FIXTURES=1 and review the change." >&2
    exit 1
  fi
}

cd "$work"
openspec init --tools none --no-animation . > /dev/null

# A capability that the team wrote by hand. Exports must leave it alone.
change=openspec/changes/add-email-notifications
mkdir -p "$change/specs/notifications"
cat > "$change/.openspec.yaml" << 'EOF'
schema: spec-driven
created: "2026-10-01"
EOF
cat > "$change/proposal.md" << 'EOF'
# Proposal

## Why

Admins miss plan changes because the product does not tell them.

## What Changes

- Send an email to each admin when the plan of the org changes.

## Capabilities

### New Capabilities

- `notifications`: Emails that the product sends to admins

## Impact

- The mailer and the billing events.
EOF
cat > "$change/specs/notifications/spec.md" << 'EOF'
# Spec Delta

## Purpose

Tells admins about changes to their org by email, so that nothing changes without them knowing.

## ADDED Requirements

### Requirement: Plan change email
The system SHALL send an email to every admin of an org when the plan of the org changes.

#### Scenario: Downgrade
- **WHEN** an admin moves the org from Business to Team
- **THEN** every admin of the org gets an email that names both plans

#### Scenario: No admins
- **WHEN** the org has no admin with a confirmed email address
- **THEN** the system sends no email and logs a warning
EOF
openspec validate add-email-notifications --no-interactive
openspec archive add-email-notifications -y > /dev/null

# Exports one version, validates it, compares it, archives it and compares
# the main specs.
step() {
  version=$1
  document=$2
  shift 2
  skopos export "$document" --to openspec "$@"
  change=$(ls openspec/changes | grep -v '^archive$')
  openspec validate "$change" --no-interactive
  check_tree "openspec/changes/$change" \
    "$core/golden/openspec-versions/v$version/openspec/changes/$change"
  openspec archive "$change" -y > /dev/null
  check_tree openspec/specs "$core/fixtures/openspec/after-v$version/openspec/specs"
}

# The first export names the capability. Later exports find it by the
# reference strings in the main specs.
step 2 "$core/fixtures/spec-v2.json" --name seat-limits
step 3 "$repo/conformance/valid/spec-all-types-v3.json"
step 4 "$core/fixtures/spec-v4.json"
step 5 "$core/fixtures/spec-v5.json"

echo "OpenSpec round trip: versions 2 to 5 validated and archived."
