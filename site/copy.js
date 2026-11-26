// A Copy button on each code block of a page, for the pages that site/build.sh
// marks with "copy" (the agent skill page). Without this script, the page
// still shows the code.
(() => {
  for (const pre of document.querySelectorAll("main pre")) {
    const box = document.createElement("div");
    box.className = "copyable";
    pre.before(box);
    box.append(pre);
    const button = document.createElement("button");
    button.type = "button";
    button.className = "copy";
    button.textContent = "Copy";
    button.addEventListener("click", async () => {
      await navigator.clipboard.writeText(pre.textContent);
      button.textContent = "Copied";
      setTimeout(() => (button.textContent = "Copy"), 1500);
    });
    box.append(button);
  }
})();
