// A CodeMirror 6 editor for JSON, YAML and Markdown, with the problems from
// core shown in the text.
import { autocompletion, closeBrackets, closeBracketsKeymap, completionKeymap, type CompletionSource } from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { json } from "@codemirror/lang-json";
import { markdown } from "@codemirror/lang-markdown";
import { yaml } from "@codemirror/lang-yaml";
import { bracketMatching, foldGutter, HighlightStyle, indentOnInput, syntaxHighlighting } from "@codemirror/language";
import { lintGutter, setDiagnostics, type Diagnostic } from "@codemirror/lint";
import { highlightSelectionMatches, searchKeymap } from "@codemirror/search";
import { Annotation, Compartment, EditorSelection, EditorState } from "@codemirror/state";
import { drawSelection, EditorView, highlightActiveLine, highlightActiveLineGutter, keymap, lineNumbers, placeholder as placeholderExt } from "@codemirror/view";
import { tags as t } from "@lezer/highlight";
import { useEffect, useRef } from "preact/hooks";
import type { Format, Mark } from "./doc";
import { completions } from "./complete";

/** Marks a change that came from the value prop, not from the person. */
const external = Annotation.define<boolean>();

const highlight = HighlightStyle.define([
  { tag: [t.propertyName, t.definition(t.propertyName)], color: "var(--syn-key)" },
  { tag: [t.string, t.special(t.string)], color: "var(--syn-string)" },
  { tag: [t.number, t.bool, t.null, t.atom], color: "var(--syn-atom)" },
  { tag: [t.comment, t.meta, t.processingInstruction], color: "var(--muted)" },
  { tag: t.heading, fontWeight: "600", color: "var(--ink)" },
  { tag: t.heading1, fontWeight: "650" },
  { tag: [t.list, t.punctuation, t.separator, t.bracket], color: "var(--muted)" },
  { tag: t.emphasis, fontStyle: "italic" },
  { tag: t.strong, fontWeight: "600" },
  { tag: t.link, color: "var(--syn-key)", textDecoration: "underline" },
]);

const theme = EditorView.theme({
  "&": { height: "100%", fontSize: "13px", backgroundColor: "var(--code-bg)", color: "var(--ink)" },
  "&.cm-focused": { outline: "none" },
  ".cm-scroller": { fontFamily: "var(--mono)", lineHeight: "1.6" },
  ".cm-content": { caretColor: "var(--dot)", padding: "10px 0" },
  ".cm-gutters": { backgroundColor: "var(--code-bg)", color: "var(--faint)", border: "none" },
  ".cm-activeLine": { backgroundColor: "color-mix(in oklab, var(--ink) 4%, transparent)" },
  ".cm-activeLineGutter": { backgroundColor: "transparent", color: "var(--ink)" },
  ".cm-selectionBackground, &.cm-focused .cm-selectionBackground, ::selection": { backgroundColor: "color-mix(in oklab, var(--dot) 22%, transparent) !important" },
  ".cm-cursor": { borderLeftColor: "var(--dot)", borderLeftWidth: "2px" },
  ".cm-placeholder": { color: "var(--faint)" },
  ".cm-lintRange-error": { backgroundImage: "none", textDecoration: "underline wavy var(--bad)", textUnderlineOffset: "3px", textDecorationSkipInk: "none" },
  ".cm-lintRange-warning": { backgroundImage: "none", textDecoration: "underline wavy var(--warn)", textUnderlineOffset: "3px" },
  ".cm-tooltip": { backgroundColor: "var(--surface)", border: "1px solid var(--line)", borderRadius: "6px", boxShadow: "0 6px 20px -8px rgb(0 0 0 / .25)", fontFamily: "var(--sans)", overflow: "hidden" },
  ".cm-tooltip-lint": { padding: "0" },
  ".cm-diagnostic": { padding: "6px 10px", borderLeft: "3px solid var(--bad)", fontSize: "13px" },
  ".cm-diagnostic-warning": { borderLeftColor: "var(--warn)" },
  ".cm-tooltip-autocomplete > ul": { fontFamily: "var(--mono)", fontSize: "12.5px" },
  ".cm-tooltip-autocomplete > ul > li[aria-selected]": { backgroundColor: "var(--code-bg)", color: "var(--ink)" },
  ".cm-completionDetail": { fontFamily: "var(--sans)", color: "var(--muted)", fontStyle: "normal", marginLeft: "1em" },
  ".cm-completionInfo": { fontFamily: "var(--sans)", fontSize: "13px", maxWidth: "320px", padding: "6px 10px" },
  ".cm-foldGutter span": { color: "var(--faint)" },
  ".cm-lint-marker": { width: "0.8em", height: "0.8em" },
  ".cm-highlight-flash": { backgroundColor: "color-mix(in oklab, var(--dot) 18%, transparent)", transition: "background-color 1s" },
});

function language(format: Format) {
  if (format === "json") return json();
  if (format === "markdown") return markdown();
  return yaml();
}

export interface EditorHandle {
  /** Selects a range and scrolls it into view. */
  reveal(from: number, to: number): void;
  /** Scrolls a place into view without moving the focus. */
  show(pos: number): void;
  focus(): void;
}

interface Props {
  value: string;
  format: Format;
  onChange?: (text: string) => void;
  marks?: Mark[];
  readOnly?: boolean;
  placeholder?: string;
  label: string;
  handle?: (h: EditorHandle | null) => void;
  complete?: boolean;
  /** Called with the cursor position when the person moves it. */
  onCursor?: (pos: number) => void;
}

export function Editor({ value, format, onChange, marks = [], readOnly = false, placeholder = "", label, handle, complete = false, onCursor }: Props) {
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | undefined>(undefined);
  const lang = useRef(new Compartment());
  const ro = useRef(new Compartment());
  const changed = useRef(onChange);
  changed.current = onChange;
  const cursor = useRef(onCursor);
  cursor.current = onCursor;

  useEffect(() => {
    const sources: CompletionSource[] = complete ? [completions] : [];
    const v = new EditorView({
      parent: host.current!,
      state: EditorState.create({
        doc: value,
        extensions: [
          lineNumbers(),
          foldGutter(),
          lintGutter(),
          history(),
          drawSelection(),
          indentOnInput(),
          bracketMatching(),
          closeBrackets(),
          highlightActiveLine(),
          highlightActiveLineGutter(),
          highlightSelectionMatches(),
          syntaxHighlighting(highlight),
          EditorView.lineWrapping,
          autocompletion({ override: sources.length ? sources : null, activateOnTyping: true }),
          keymap.of([...closeBracketsKeymap, ...defaultKeymap, ...searchKeymap, ...historyKeymap, ...completionKeymap, indentWithTab]),
          placeholderExt(placeholder),
          EditorView.contentAttributes.of({ "aria-label": label }),
          lang.current.of(language(format)),
          ro.current.of([EditorState.readOnly.of(readOnly), EditorView.editable.of(!readOnly)]),
          theme,
          EditorView.updateListener.of((u) => {
            const fromProp = u.transactions.some((tr) => tr.annotation(external));
            if (u.docChanged && !fromProp) changed.current?.(u.state.doc.toString());
            if (u.selectionSet && !fromProp && u.view.hasFocus) cursor.current?.(u.state.selection.main.head);
          }),
        ],
      }),
    });
    view.current = v;
    handle?.({
      reveal(from, to) {
        const len = v.state.doc.length;
        v.dispatch({
          selection: EditorSelection.range(Math.min(from, len), Math.min(to, len)),
          effects: EditorView.scrollIntoView(Math.min(from, len), { y: "center" }),
        });
        v.focus();
      },
      focus: () => v.focus(),
      show(pos) {
        v.dispatch({ effects: EditorView.scrollIntoView(Math.min(pos, v.state.doc.length), { y: "start", yMargin: 40 }) });
      },
    });
    return () => {
      handle?.(null);
      v.destroy();
    };
  }, []);

  useEffect(() => {
    const v = view.current!;
    const old = v.state.doc.toString();
    if (old !== value) {
      // Replace only the part that changed, so the scroll position and the
      // cursor stay where they are.
      let start = 0;
      const max = Math.min(old.length, value.length);
      while (start < max && old.charCodeAt(start) === value.charCodeAt(start)) start++;
      let end = 0;
      while (end < max - start && old.charCodeAt(old.length - 1 - end) === value.charCodeAt(value.length - 1 - end)) end++;
      v.dispatch({ changes: { from: start, to: old.length - end, insert: value.slice(start, value.length - end) }, annotations: external.of(true) });
    }
  }, [value]);

  useEffect(() => {
    view.current!.dispatch({ effects: lang.current.reconfigure(language(format)) });
  }, [format]);

  useEffect(() => {
    view.current!.dispatch({ effects: ro.current.reconfigure([EditorState.readOnly.of(readOnly), EditorView.editable.of(!readOnly)]) });
  }, [readOnly]);

  useEffect(() => {
    const v = view.current!;
    const len = v.state.doc.length;
    const diagnostics: Diagnostic[] = marks.map((m) => {
      const from = Math.min(m.from, len);
      return {
        from,
        to: Math.min(Math.max(m.to, from), len),
        severity: m.severity,
        message: m.message,
        source: m.rule ?? undefined,
      };
    });
    v.dispatch(setDiagnostics(v.state, diagnostics));
  }, [marks, value]);

  return <div class="editor" ref={host} />;
}
