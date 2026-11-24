// The workbench: one tab per tool, each with its own URL. Switching tabs
// keeps the work of the other tabs.
import { render } from "preact";
import { useEffect, useState } from "preact/hooks";
import { core, next, ready } from "./core";
import { tools, type Go, type Handoff, type ToolId } from "./nav";
import { Validate } from "./tools/validate";
import { Author } from "./tools/author";
import { Compare } from "./tools/compare";
import { AgentFile } from "./tools/agent-file";
import { History } from "./tools/history";
import { Merge } from "./tools/merge";
import { Home } from "./tools/home";
import "./tools.css";

const titles: Record<ToolId, string> = {
  home: "OpenSkopos tools",
  validate: "Validate an OpenSkopos document",
  author: "Write an OpenSkopos document",
  compare: "Compare two OpenSkopos versions",
  history: "The history of OpenSkopos documents",
  merge: "Merge two OpenSkopos drafts",
  "agent-file": "Make an agent file from OpenSkopos documents",
};

const pathOf = (id: ToolId) => (id === "home" ? "/tools/" : `/tools/${id}/`);
const toolOf = (path: string): ToolId => (tools.find((t) => path.startsWith(`/tools/${t.id}/`))?.id ?? "home") as ToolId;

function App({ initial }: { initial: ToolId }) {
  const [active, setActive] = useState<ToolId>(initial);
  const [seen, setSeen] = useState<Set<ToolId>>(new Set([initial]));
  const [handoff, setHandoff] = useState<Handoff>();

  useEffect(() => {
    const onPop = () => show(toolOf(location.pathname));
    addEventListener("popstate", onPop);
    return () => removeEventListener("popstate", onPop);
  }, []);

  function show(id: ToolId) {
    setActive(id);
    setSeen((s) => new Set(s).add(id));
    document.title = `${titles[id]} · OpenSkopos`;
  }

  const go: Go = (id, file) => {
    if (file) setHandoff((h) => ({ tool: id, file, n: (h?.n ?? 0) + 1 }));
    if (id !== active) {
      history.pushState(null, "", pathOf(id));
      show(id);
      scrollTo(0, 0);
    }
  };

  const panel = (id: ToolId, el: preact.JSX.Element) => seen.has(id) && (
    <section class="tool" id={`tool-${id}`} hidden={active !== id} aria-labelledby={`tab-${id}`}>
      {el}
    </section>
  );
  const incoming = (id: ToolId) => (handoff?.tool === id ? handoff : undefined);

  return (
    <div class="workbench">
      <div class="toolbar">
        <nav class="tabs" aria-label="Tools">
          {tools.filter((t) => (t.id !== "history" || next.history || active === "history") && (t.id !== "merge" || next.merge || active === "merge")).map((t) => (
            <a
              id={`tab-${t.id}`}
              href={pathOf(t.id)}
              aria-current={active === t.id ? "page" : undefined}
              onClick={(e) => {
                if (e.metaKey || e.ctrlKey || e.shiftKey) return;
                e.preventDefault();
                go(t.id);
              }}
            >
              {t.label}
            </a>
          ))}
        </nav>
        <span class="local" title="The tools run the OpenSkopos reference implementation, built to WebAssembly, in this page. Your documents never leave your browser.">
          <i aria-hidden="true" />
          Runs in your browser · <code>@openskopos/core</code> {core.version()}
        </span>
      </div>
      {panel("home", <Home go={go} />)}
      {panel("validate", <Validate go={go} incoming={incoming("validate")} />)}
      {panel("author", <Author go={go} incoming={incoming("author")} />)}
      {panel("compare", <Compare go={go} incoming={incoming("compare")} />)}
      {panel("history", <History go={go} incoming={incoming("history")} />)}
      {panel("merge", <Merge go={go} incoming={incoming("merge")} />)}
      {panel("agent-file", <AgentFile go={go} incoming={incoming("agent-file")} />)}
    </div>
  );
}

const root = document.getElementById("app")!;
root.innerHTML = '<div class="loading">Loading the OpenSkopos engine…</div>';
ready.then(
  () => {
    root.innerHTML = "";
    render(<App initial={toolOf(location.pathname)} />, root);
  },
  (e) => {
    root.innerHTML = `<div class="loading error">The OpenSkopos engine did not load: ${String(e?.message ?? e)}. The tools need a browser with WebAssembly.</div>`;
  },
);
