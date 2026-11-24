import { tools, type Go } from "../nav";

export function Home({ go }: { go: Go }) {
  return (
    <div class="home-tools">
      <h1>Tools for OpenSkopos documents</h1>
      <p class="lead">
        Check, write and compare the documents that record what a team decided before an AI agent builds the work. The tools run the reference implementation in this page, so they give the same results as the{" "}
        <a href="/docs/cli/">skopos command line tool</a>. Your documents never leave your browser.
      </p>
      <ul class="tool-cards">
        {tools.map((t) => (
          <li>
            <a
              href={`/tools/${t.id}/`}
              onClick={(e) => {
                if (e.metaKey || e.ctrlKey || e.shiftKey) return;
                e.preventDefault();
                go(t.id);
              }}
            >
              <span class="name">{t.label}</span>
              <span class="blurb">{t.blurb}</span>
            </a>
          </li>
        ))}
      </ul>
      <p class="muted">Next: History, to read a chain of versions and see which decisions are in force now, and Merge, to combine two drafts into one document.</p>
    </div>
  );
}
