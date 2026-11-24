// The version picker of the install page (/docs/install/). The page itself
// shows the commands for the latest release, so it works without this
// script. The picker changes the commands to install an earlier release.
// site/build.sh puts the x.y versions, newest first, in data-versions, each
// with its latest full version for the download links: "0.2=0.2.0 0.1=0.1.0".
// The picker shows only x.y.
(() => {
  const versions = document.currentScript.dataset.versions.split(" ").map((p) => p.split("="));
  const latest = versions[0][0];
  const full = Object.fromEntries(versions);
  const main = document.querySelector("main");
  const firstSection = main.querySelector("h2");
  if (!firstSection || versions.length < 2) return;

  // The commands: code that starts with a program name, not names in prose.
  const codes = [...main.querySelectorAll("code")]
    .filter((el) => /^(curl|powershell|cargo|npm|npx) /.test(el.textContent))
    .map((el) => ({ el, text: el.textContent }));
  const releaseLinks = [...main.querySelectorAll('a[href="https://github.com/openskopos/openskopos/releases"]')];
  const brew = [...main.querySelectorAll("code")].find((el) => el.textContent.startsWith("brew install"));

  // Homebrew has only the latest release.
  const brewNote = document.createElement("p");
  brewNote.className = "muted";
  brewNote.textContent = "Homebrew installs only the latest release. Use another way below.";
  brewNote.hidden = true;
  brew?.closest("pre")?.after(brewNote);

  const picker = document.createElement("p");
  picker.className = "picker";
  picker.innerHTML =
    '<label>Version <select></select></label> <a href="/changelog/">What changed</a>';
  const select = picker.querySelector("select");
  const changes = picker.querySelector("a");
  for (const [v] of versions) {
    select.add(new Option(v === latest ? `${v} (latest)` : v, v));
  }
  firstSection.before(picker);

  function show(v) {
    const old = v !== latest;
    const f = full[v];
    for (const { el, text } of codes) {
      el.textContent = !old
        ? text
        : text
            .replace("/releases/latest/download/", `/releases/download/v${f}/`)
            .replace(/(cargo install --locked skopos)$/m, `$1 --version ~${v}`)
            .replace(/@openskopos\/cli\b(?!@)/g, `@openskopos/cli@${v}`);
    }
    for (const a of releaseLinks) {
      a.href = old
        ? `https://github.com/openskopos/openskopos/releases/tag/v${f}`
        : "https://github.com/openskopos/openskopos/releases";
    }
    brewNote.hidden = !old;
    changes.href = `/changelog/#v${v}`;
  }
  select.addEventListener("change", () => show(select.value));
  show(latest);
})();
