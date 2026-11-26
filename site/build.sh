#!/bin/sh
# Builds openskopos.org into a folder (default: _site). Run it from the root
# of the repository. It needs pandoc 3.11 or later, and Node 20 or later for
# the tools at /tools/.
#
# Besides the HTML pages, it writes a Markdown copy of each page made from
# Markdown (index.md next to index.html), sitemap.xml, llms.txt and
# llms-full.txt. site/robots.txt allows every crawler.
#
# The schema files are not part of this build. The Deploy site workflow adds
# each tagged schema file to the folder.
set -eu

out="${1:-_site}"
rm -rf "$out"
mkdir -p "$out"

# The static files. The pandoc folder and this script are build files only.
cp -R site/. "$out/"
rm -rf "$out/pandoc" "$out/build.sh" "$out/tools"

# The tools at /tools/ are a Vite app in site/tools. It runs the WebAssembly
# build of skopos-core from the @openskopos/core package.
abs_out=$(cd "$out" && pwd)
(
  cd site/tools
  [ -d node_modules ] || npm ci --no-audit --no-fund
  npx tsc --noEmit
  npx vite build --logLevel warn --outDir "$abs_out/tools" --emptyOutDir
)
echo "site/tools -> /tools/"

# The image for links on social sites.
cp brand/avatar-512-light.png "$out/og.png"

# Each line: the Markdown file, the folder on the site, the page title,
# "toc" if the page gets a table of contents (the specification has its own),
# and the description for search engines (can be empty).
pages='
spec/0.2/README.md|spec/0.2|Specification 0.2||The OpenSkopos specification 0.2: the file format for the decisions that a team makes before an AI agent builds the work.
spec/0.1/README.md|spec/0.1|Specification 0.1||The OpenSkopos specification 0.1: the file format for the decisions that a team makes before an AI agent builds the work.
RELEASES.md|changelog|Changelog||What each OpenSkopos release added, changed and removed: the format, the skopos command line tool, the Rust crates and the npm packages.
spec/CHANGELOG.md|spec/changelog|Changelog of the format||The changes to the OpenSkopos format, by format version.
docs/README.md|docs|Documentation||How to use OpenSkopos documents and the skopos tool for spec-driven development.
docs/install.md|docs/install|Install the skopos tool||Install the skopos command line tool on macOS, Linux or Windows, with Homebrew, an install script, Cargo or npm.
docs/mcp.md|docs/mcp|Use OpenSkopos from an agent|toc|Connect Claude Code, Codex, Cursor, VS Code or another AI agent to the free OpenSkopos MCP server to check, fix, convert and merge documents.
docs/editors.md|docs/editors|Editor support||Validation and completion for .skopos.json and .skopos.yaml files in VS Code and other editors.
docs/exporters.md|docs/exporters|Export to OpenSpec, Kiro and Spec Kit|toc|Export an OpenSkopos document to OpenSpec, Kiro or GitHub Spec Kit, and see where each item type goes in each tool.
docs/render.md|docs/render|Readable views of a document|toc|Show an OpenSkopos document as text in the terminal or as an HTML page, and style the HTML with CSS custom properties.
crates/skopos/README.md|docs/cli|The skopos command line tool|toc|The skopos commands: check, fmt, convert, import, ids, export, diff, history, trace, merge, conflicts, resolve and render.
crates/skopos-core/README.md|docs/rust|The skopos-core Rust library||Read, check, write, compare, merge and render OpenSkopos documents in Rust with the reference implementation.
npm/core/README.md|docs/javascript|The @openskopos/core npm package||Read, check, write, compare, merge and render OpenSkopos documents in Node and in browsers, with the reference implementation built to WebAssembly.
npm/cli/README.md|docs/npm-cli|The @openskopos/cli npm package||The skopos command line tool for Node on every platform, with the same commands and output as the native tool.
conformance/README.md|docs/conformance|Conformance cases||The documents that every OpenSkopos implementation must accept or reject, and the cases for Markdown, histories and merges.
'

# The x.y versions, newest first, from the headings of RELEASES.md, each with
# its latest full version for the download links, such as "0.2=0.2.0 0.1=0.1.0".
# The newest x.y uses the version of the packages. An older x.y uses its
# newest tag v<x.y>.<z>, or <x.y>.0 without tags (a shallow checkout). The
# install page gets them for its version picker (site/install.js).
package=$(sed -n 's/^version = "\(.*\)"$/\1/p' crates/skopos/Cargo.toml)
versions=$(
  for minor in $(sed -n 's/^## \([0-9][0-9]*\.[0-9][0-9]*\)$/\1/p' RELEASES.md); do
    case "$package" in
      "$minor".*) full=$package ;;
      *) full=$(git tag --list "v$minor.*" 2>/dev/null | sed 's/^v//' | sort -t. -k3,3n | tail -n 1) ;;
    esac
    printf '%s=%s ' "$minor" "${full:-$minor.0}"
  done | sed 's/ $//'
)

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
  # schema.org breadcrumbs: Home, then Documentation for the pages under
  # docs/, then the page.
  crumb() { printf '{"@type":"ListItem","position":%s,"name":"%s","item":"https://openskopos.org%s"}' "$1" "$2" "$3"; }
  case "$folder" in
    docs/*) crumbs="$(crumb 1 OpenSkopos /),$(crumb 2 Documentation /docs/),$(crumb 3 "$title" "/$folder/")" ;;
    *) crumbs="$(crumb 1 OpenSkopos /),$(crumb 2 "$title" "/$folder/")" ;;
  esac
  set -- "$@" --variable breadcrumbs="{\"@context\":\"https://schema.org\",\"@type\":\"BreadcrumbList\",\"itemListElement\":[$crumbs]}"
  if [ "$folder" = docs/install ]; then
    set -- "$@" --metadata versions="$versions"
  fi
  if [ "$toc" = toc ]; then
    set -- "$@" --toc --toc-depth 3
  fi
  pandoc "$@"
  # The same page as Markdown, for AI models and other tools, with the links
  # to the site as full URLs.
  pandoc "$source" --from gfm --to gfm --wrap none \
    --lua-filter site/pandoc/links.lua |
    sed "s|](/|](https://openskopos.org/|g" > "$out/$folder/index.md"
  echo "$source -> /$folder/"
done

site=https://openskopos.org
# The format version of the latest release, such as 0.2.
format=$(echo "$versions" | cut -d' ' -f1 | cut -d= -f1)

# The title and description of a page that is not made from Markdown.
html_title() { sed -n 's|.*<title>\(.*\)</title>.*|\1|p' "$1" | sed 's| · OpenSkopos$||'; }
html_description() { sed -n 's|.*<meta name="description" content="\([^"]*\)">.*|\1|p' "$1"; }

# sitemap.xml: every page of the site, with the date of the last commit that
# changed its source.
lastmod() { git log -1 --format=%cs -- "$@" 2>/dev/null || true; }
url() {
  printf '  <url><loc>%s%s</loc>' "$site" "$1"
  [ -z "$2" ] || printf '<lastmod>%s</lastmod>' "$2"
  printf '</url>\n'
}
{
  echo '<?xml version="1.0" encoding="UTF-8"?>'
  echo '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">'
  url / "$(lastmod site/index.html)"
  url /tools/ "$(lastmod site/tools)"
  for page in "$out"/tools/*/index.html; do
    url "/tools/$(basename "$(dirname "$page")")/" "$(lastmod site/tools)"
  done
  echo "$pages" | while IFS='|' read -r source folder title toc description; do
    [ -n "$source" ] || continue
    url "/$folder/" "$(lastmod "$source")"
  done
  echo '</urlset>'
} > "$out/sitemap.xml"
echo "sitemap.xml: $(grep -c '<loc>' "$out/sitemap.xml") pages"

# llms.txt (https://llmstxt.org): a short guide to the site for AI models,
# with links to the Markdown copy of each page. llms-full.txt has the
# Markdown of the current specification and every documentation page in one
# file.
link() { printf -- '- [%s](%s)' "$1" "$2"; [ -z "${3:-}" ] || printf ': %s' "$3"; printf '\n'; }
# The pages whose folder matches one of the patterns.
pages_in() {
  echo "$pages" | while IFS='|' read -r source folder title toc description; do
    [ -n "$source" ] || continue
    for pattern in "$@"; do
      case "$folder" in $pattern)
        link "$title" "$site/$folder/index.md" "$description"
        break
        ;;
      esac
    done
  done
}
{
  echo "# OpenSkopos"
  echo
  echo "> OpenSkopos is an open, vendor-neutral file format for the decisions that a team makes before an AI agent builds the work: goals, requirements, constraints, assumptions, risks, open questions and decisions, each with a stable ID. The skopos command line tool and the skopos-core library check, convert, compare, merge and render documents, and export them to OpenSpec, GitHub Spec Kit and Kiro."
  echo
  echo "The current version is $format. The tools read and write format $format. A document names its format version in \`\$schema\`, for example \`$site/schema/artifact/$format.json\`. Files end in \`.skopos.json\`, \`.skopos.yaml\` or \`.skopos.md\`. Everything is under the Apache License 2.0."
  echo
  echo "## Specification"
  echo
  pages_in "spec/$format"
  link "JSON Schema $format" "$site/schema/artifact/$format.json" "The JSON Schema of format $format."
  pages_in "spec/changelog"
  echo
  echo "## Documentation"
  echo
  pages_in docs "docs/*" changelog
  echo
  echo "## Tools"
  echo
  for page in "$out"/tools/index.html "$out"/tools/*/index.html; do
    path=${page#"$out"}
    link "$(html_title "$page")" "$site${path%index.html}" "$(html_description "$page")"
  done
  echo
  echo "## Optional"
  echo
  link "All documentation in one file" "$site/llms-full.txt" "The specification $format and every documentation page as Markdown."
  echo "$pages" | while IFS='|' read -r source folder title toc description; do
    case "$folder" in spec/[0-9]*) [ "$folder" = "spec/$format" ] || link "$title" "$site/$folder/index.md" "An earlier format version." ;; esac
  done
  link "Source code" "https://github.com/openskopos/openskopos"
} > "$out/llms.txt"
{
  echo "# OpenSkopos"
  echo "$pages" | while IFS='|' read -r source folder title toc description; do
    [ -n "$source" ] || continue
    case "$folder" in spec/[0-9]*) [ "$folder" = "spec/$format" ] || continue ;; esac
    printf '\n\n<!-- %s/%s/ -->\n\n' "$site" "$folder"
    cat "$out/$folder/index.md"
  done
} > "$out/llms-full.txt"
echo "llms.txt, llms-full.txt: $(wc -c < "$out/llms-full.txt") bytes"

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
