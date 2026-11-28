-- Pandoc filter for the site build. It changes the relative links in a
-- Markdown file of the repository:
--
-- * A link to a file that the site renders goes to the page on the site.
-- * A link to any other file goes to the file on GitHub.
--
-- So the same Markdown files work on GitHub and on the site.
--
-- In the changelog (RELEASES.md), it gives each version heading the stable
-- ID v<version>, such as v0.2.0, and each section under it an ID such as
-- v0.2.0-added.
--
-- It also marks the code in tables that must not break across lines (see
-- Table at the end of this file).

local pages = {
  ["spec/0.2/README.md"] = "/spec/0.2/",
  ["spec/0.1/README.md"] = "/spec/0.1/",
  ["spec/CHANGELOG.md"] = "/spec/changelog/",
  ["RELEASES.md"] = "/changelog/",
  ["docs/README.md"] = "/docs/",
  ["docs/install.md"] = "/docs/install/",
  ["docs/mcp.md"] = "/docs/mcp/",
  ["docs/editors.md"] = "/docs/editors/",
  ["docs/exporters.md"] = "/docs/exporters/",
  ["docs/compaction.md"] = "/docs/compaction/",
  ["docs/render.md"] = "/docs/render/",
  ["docs/skill.md"] = "/docs/skill/",
  ["skills/openskopos/SKILL.md"] = "/skills/openskopos/SKILL.md",
  ["crates/skopos/README.md"] = "/docs/cli/",
  ["crates/skopos-core/README.md"] = "/docs/rust/",
  ["npm/core/README.md"] = "/docs/javascript/",
  ["npm/cli/README.md"] = "/docs/npm-cli/",
  ["conformance/README.md"] = "/docs/conformance/",
  ["schema/artifact/0.2.json"] = "/schema/artifact/0.2.json",
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

-- In a table, code with no spaces, such as a field name or a rule name, must
-- not break across lines. The class "nowrap" in style.css does this.
function Table(tbl)
  return tbl:walk({
    Code = function(code)
      if not code.text:find("%s") then
        code.classes:insert("nowrap")
      end
      return code
    end,
  })
end

-- The page of the agent skill ends with the whole text of the skill, so
-- people can read and copy it on the site.
if source == "docs/skill.md" then
  function Pandoc(doc)
    local file = assert(io.open("skills/openskopos/SKILL.md"))
    doc.blocks:insert(pandoc.CodeBlock(file:read("a"):gsub("\n$", ""), pandoc.Attr("", { "markdown" })))
    file:close()
    return doc
  end
end

-- The stable IDs of the changelog headings.
if source == "RELEASES.md" then
  local version = nil
  function Header(h)
    local text = pandoc.utils.stringify(h.content)
    if h.level == 2 then
      version = "v" .. text
      h.identifier = version
    elseif h.level == 3 and version then
      h.identifier = version .. "-" .. text:lower():gsub("%s+", "-")
    end
    return h
  end
end
