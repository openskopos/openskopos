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

# Each line: the Markdown file, the folder on the site, the page title, and
# "toc" if the page gets a table of contents. The specification has its own.
pages='
spec/0.1/README.md|spec/0.1|Specification 0.1|
spec/CHANGELOG.md|spec/changelog|Changelog of the format|
docs/README.md|docs|Documentation|
docs/install.md|docs/install|Install the skopos tool|
docs/editors.md|docs/editors|Editor support|
docs/exporters.md|docs/exporters|Exporters|toc
crates/skopos/README.md|docs/cli|The skopos command line tool|toc
crates/skopos-core/README.md|docs/rust|The skopos-core Rust library|
npm/core/README.md|docs/javascript|The @openskopos/core npm package|
npm/cli/README.md|docs/npm-cli|The @openskopos/cli npm package|
conformance/README.md|docs/conformance|Conformance cases|
'

echo "$pages" | while IFS='|' read -r source folder title toc; do
  [ -n "$source" ] || continue
  mkdir -p "$out/$folder"
  set -- "$source" --from gfm --to html5 --standalone \
    --template site/pandoc/template.html \
    --lua-filter site/pandoc/links.lua \
    --syntax-highlighting none \
    --metadata pagetitle="$title" \
    --metadata source="$source" \
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
