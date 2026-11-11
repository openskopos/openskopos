-- Pandoc filter for the site build. It changes the relative links in a
-- Markdown file of the repository:
--
-- * A link to a file that the site renders goes to the page on the site.
-- * A link to any other file goes to the file on GitHub.
--
-- So the same Markdown files work on GitHub and on the site.

local pages = {
  ["spec/0.1/README.md"] = "/spec/0.1/",
  ["spec/CHANGELOG.md"] = "/spec/changelog/",
  ["docs/README.md"] = "/docs/",
  ["docs/install.md"] = "/docs/install/",
  ["docs/editors.md"] = "/docs/editors/",
  ["docs/exporters.md"] = "/docs/exporters/",
  ["crates/skopos/README.md"] = "/docs/cli/",
  ["crates/skopos-core/README.md"] = "/docs/rust/",
  ["npm/core/README.md"] = "/docs/javascript/",
  ["npm/cli/README.md"] = "/docs/npm-cli/",
  ["conformance/README.md"] = "/docs/conformance/",
  ["schema/artifact/0.1.json"] = "/schema/artifact/0.1.json",
}

local github = "https://github.com/openskopos/openskopos/"
local source = PANDOC_STATE.input_files[1]

-- Joins the parts of a path and removes "." and "..".
local function normalize(path)
  local parts = {}
  for part in path:gmatch("[^/]+") do
    if part == ".." then
      table.remove(parts)
    elseif part ~= "." then
      table.insert(parts, part)
    end
  end
  return table.concat(parts, "/")
end

function Link(link)
  local target = link.target
  -- Keep links with a scheme (https:, mailto:), links in the same page and
  -- links from the root of the site.
  if target:match("^%a[%w+.-]*:") or target:match("^#") or target:match("^/") then
    return nil
  end
  local path, fragment = target:match("^([^#]*)(.*)$")
  local folder = source:match("^(.*)/") or ""
  local file = normalize(folder .. "/" .. path)
  if pages[file] then
    link.target = pages[file] .. fragment
  elseif path:match("/$") or file == "" then
    link.target = github .. "tree/main/" .. file .. fragment
  else
    link.target = github .. "blob/main/" .. file .. fragment
  end
  return link
end
