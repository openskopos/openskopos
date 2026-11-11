#!/bin/sh
# Builds openskopos.org into a folder (default: _site). Run it from the root
# of the repository. It needs pandoc 3.11 or later.
#
# The schema files are not part of this build. The Deploy site workflow adds
# each tagged schema file to the folder.
set -eu

out="${1:-_site}"
rm -rf "$out"
mkdir -p "$out"

# The static files. The pandoc folder and this script are build files only.
cp -R site/. "$out/"
rm -rf "$out/pandoc" "$out/build.sh"

# The image for links on social sites.
cp brand/avatar-512-light.png "$out/og.png"

# Each line: the Markdown file, the folder on the site, the page title,
# "toc" if the page gets a table of contents (the specification has its own),
# and the description for search engines (can be empty).
pages='
spec/0.1/README.md|spec/0.1|Specification 0.1||The OpenSkopos specification 0.1: the file format for the decisions that a team makes before an AI agent builds the work.
spec/CHANGELOG.md|spec/changelog|Changelog of the format||The changes to the OpenSkopos format, by format version.
docs/README.md|docs|Documentation||How to use OpenSkopos documents and the skopos tool for spec-driven development.
docs/install.md|docs/install|Install the skopos tool||Install the skopos command line tool on macOS, Linux or Windows, with Homebrew, an install script, Cargo or npm.
docs/editors.md|docs/editors|Editor support||Validation and completion for .skopos.json and .skopos.yaml files in VS Code and other editors.
docs/exporters.md|docs/exporters|Export to OpenSpec, Kiro and Spec Kit|toc|Export an OpenSkopos document to OpenSpec, Kiro or GitHub Spec Kit, and see where each item type goes in each tool.
crates/skopos/README.md|docs/cli|The skopos command line tool|toc|The skopos commands: check, fmt, convert, import, ids, export and diff.
crates/skopos-core/README.md|docs/rust|The skopos-core Rust library||
npm/core/README.md|docs/javascript|The @openskopos/core npm package||
npm/cli/README.md|docs/npm-cli|The @openskopos/cli npm package||
conformance/README.md|docs/conformance|Conformance cases||
'

echo "$pages" | while IFS='|' read -r source folder title toc description; do
  [ -n "$source" ] || continue
  mkdir -p "$out/$folder"
  set -- "$source" --from gfm --to html5 --standalone \
    --template site/pandoc/template.html \
    --lua-filter site/pandoc/links.lua \
    --syntax-highlighting none --wrap none \
    --metadata pagetitle="$title" \
    --metadata source="$source" \
    --metadata folder="$folder" \
    --metadata description="$description" \
    --output "$out/$folder/index.html"
  if [ "$toc" = toc ]; then
    set -- "$@" --toc --toc-depth 3
  fi
  pandoc "$@"
  echo "$source -> /$folder/"
done

# Make sure that each link from the root of the site goes to a file of the
# site. The schema files are left out: the deploy adds them from their tags.
status=0
for link in $(grep -rhoE 'href="/[^"#]*' "$out" | sed 's/^href="//' | sort -u); do
  case "$link" in
    /schema/*) continue ;;
  esac
  if [ ! -e "$out$link" ] && [ ! -e "$out${link}index.html" ]; then
    echo "Broken link: $link" >&2
    status=1
  fi
done
exit $status
