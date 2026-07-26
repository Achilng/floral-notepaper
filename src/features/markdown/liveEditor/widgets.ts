import { EditorView, WidgetType } from "@codemirror/view";
import { foldEffect, foldedRanges, unfoldEffect } from "@codemirror/language";

const COPY_ICON =
  '<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>';
const CHECK_ICON =
  '<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>';
const CHEVRON_UP =
  '<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 15l-6-6-6 6"/></svg>';
const CHEVRON_DOWN =
  '<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 9l6 6 6-6"/></svg>';

function rangeFolded(view: EditorView, from: number, to: number): boolean {
  let folded = false;
  foldedRanges(view.state).between(from, to, () => {
    folded = true;
  });
  return folded;
}

/**
 * Top-right toolbar on a fenced code block: language · copy · fold/expand.
 * `foldFrom..foldTo` is the range collapsed when folding (everything after the
 * opening fence line); `codeFrom..codeTo` is the code text used for copy.
 */
export class CodeToolbarWidget extends WidgetType {
  constructor(
    readonly lang: string,
    readonly foldFrom: number,
    readonly foldTo: number,
    readonly codeFrom: number,
    readonly codeTo: number,
    readonly folded: boolean,
  ) {
    super();
  }
  eq(other: CodeToolbarWidget) {
    return (
      other.lang === this.lang &&
      other.foldFrom === this.foldFrom &&
      other.foldTo === this.foldTo &&
      other.codeFrom === this.codeFrom &&
      other.codeTo === this.codeTo &&
      other.folded === this.folded
    );
  }
  toDOM(view: EditorView) {
    const bar = document.createElement("div");
    bar.className = "cm-md-code-toolbar";
    bar.contentEditable = "false";
    bar.setAttribute("aria-hidden", "true");

    if (this.lang) {
      const lang = document.createElement("span");
      lang.className = "cm-md-code-lang";
      lang.textContent = this.lang;
      bar.appendChild(lang);
    }

    const copy = document.createElement("button");
    copy.type = "button";
    copy.className = "cm-md-code-btn";
    copy.title = "复制";
    copy.innerHTML = COPY_ICON;
    copy.addEventListener("mousedown", (e) => e.preventDefault());
    copy.addEventListener("click", (e) => {
      e.preventDefault();
      e.stopPropagation();
      const text = view.state.doc.sliceString(this.codeFrom, this.codeTo);
      void navigator.clipboard?.writeText(text).then(() => {
        copy.innerHTML = CHECK_ICON;
        copy.classList.add("is-copied");
        window.setTimeout(() => {
          copy.innerHTML = COPY_ICON;
          copy.classList.remove("is-copied");
        }, 1200);
      });
    });
    bar.appendChild(copy);

    const fold = document.createElement("button");
    fold.type = "button";
    fold.className = "cm-md-code-btn";
    fold.title = this.folded ? "展开" : "折叠";
    fold.innerHTML = this.folded ? CHEVRON_DOWN : CHEVRON_UP;
    fold.addEventListener("mousedown", (e) => e.preventDefault());
    fold.addEventListener("click", (e) => {
      e.preventDefault();
      e.stopPropagation();
      if (this.foldTo <= this.foldFrom) return;
      view.dispatch({
        effects: rangeFolded(view, this.foldFrom, this.foldTo)
          ? unfoldEffect.of({ from: this.foldFrom, to: this.foldTo })
          : foldEffect.of({ from: this.foldFrom, to: this.foldTo }),
      });
    });
    bar.appendChild(fold);

    return bar;
  }
  ignoreEvent() {
    return true;
  }
}

/** A round bullet replacing a `-` / `*` / `+` list marker. */
export class BulletWidget extends WidgetType {
  eq() {
    return true;
  }
  toDOM() {
    const span = document.createElement("span");
    span.className = "cm-md-bullet";
    span.textContent = "•";
    return span;
  }
  ignoreEvent() {
    return true;
  }
}

/** A rendered horizontal rule replacing a `---` / `***` / `___` line. */
export class HorizontalRuleWidget extends WidgetType {
  eq() {
    return true;
  }
  toDOM() {
    const hr = document.createElement("span");
    hr.className = "cm-md-hr";
    hr.setAttribute("aria-hidden", "true");
    return hr;
  }
  ignoreEvent() {
    return false;
  }
}

/** An interactive task-list checkbox. Toggling rewrites the source marker. */
export class CheckboxWidget extends WidgetType {
  constructor(
    readonly checked: boolean,
    readonly pos: number,
  ) {
    super();
  }
  eq(other: CheckboxWidget) {
    return other.checked === this.checked && other.pos === this.pos;
  }
  toDOM(view: EditorView) {
    const box = document.createElement("input");
    box.type = "checkbox";
    box.checked = this.checked;
    box.className = "cm-md-task";
    box.addEventListener("mousedown", (event) => {
      event.preventDefault();
      const next = this.checked ? " " : "x";
      view.dispatch({
        changes: { from: this.pos, to: this.pos + 1, insert: next },
      });
    });
    return box;
  }
  ignoreEvent() {
    return false;
  }
}

/** Renders an inline image. `resolve` maps the markdown src to a usable URL. */
export class ImageWidget extends WidgetType {
  constructor(
    readonly src: string,
    readonly alt: string,
    readonly resolve: (src: string) => string,
  ) {
    super();
  }
  eq(other: ImageWidget) {
    return other.src === this.src && other.alt === this.alt;
  }
  toDOM() {
    const img = document.createElement("img");
    img.src = this.resolve(this.src);
    img.alt = this.alt;
    img.loading = "lazy";
    img.className = "cm-md-image";
    return img;
  }
  ignoreEvent() {
    return true;
  }
}

interface TableModel {
  aligns: Array<"left" | "center" | "right" | null>;
  header: string[];
  rows: string[][];
}

function splitRow(line: string): string[] {
  let s = line.trim();
  if (s.startsWith("|")) s = s.slice(1);
  if (s.endsWith("|")) s = s.slice(0, -1);
  const cells: string[] = [];
  let current = "";
  for (let i = 0; i < s.length; i++) {
    const ch = s[i];
    if (ch === "\\" && i + 1 < s.length) {
      current += ch + s[i + 1];
      i++;
      continue;
    }
    if (ch === "|") {
      cells.push(current.trim());
      current = "";
    } else {
      current += ch;
    }
  }
  cells.push(current.trim());
  return cells;
}

function parseAlign(spec: string): "left" | "center" | "right" | null {
  const s = spec.trim();
  const left = s.startsWith(":");
  const right = s.endsWith(":");
  if (left && right) return "center";
  if (right) return "right";
  if (left) return "left";
  return null;
}

export function parseTable(source: string): TableModel | null {
  const lines = source.split("\n").filter((l) => l.trim().length > 0);
  if (lines.length < 2) return null;
  const header = splitRow(lines[0]);
  const aligns = splitRow(lines[1]).map(parseAlign);
  const rows = lines.slice(2).map(splitRow);
  return { header, aligns, rows };
}

/** Renders a GFM pipe table. Clicking moves the cursor into the source. */
export class TableWidget extends WidgetType {
  constructor(
    readonly source: string,
    readonly from: number,
  ) {
    super();
  }
  eq(other: TableWidget) {
    return other.source === this.source && other.from === this.from;
  }
  toDOM(view: EditorView) {
    const wrap = document.createElement("div");
    wrap.className = "cm-md-table-wrap";
    const model = parseTable(this.source);
    if (!model) {
      wrap.textContent = this.source;
      return wrap;
    }
    const table = document.createElement("table");
    table.className = "cm-md-table";
    const thead = document.createElement("thead");
    const headRow = document.createElement("tr");
    model.header.forEach((cell, i) => {
      const th = document.createElement("th");
      th.textContent = cell;
      const a = model.aligns[i];
      if (a) th.style.textAlign = a;
      headRow.appendChild(th);
    });
    thead.appendChild(headRow);
    table.appendChild(thead);
    const tbody = document.createElement("tbody");
    model.rows.forEach((row) => {
      const tr = document.createElement("tr");
      model.header.forEach((_, i) => {
        const td = document.createElement("td");
        td.textContent = row[i] ?? "";
        const a = model.aligns[i];
        if (a) td.style.textAlign = a;
        tr.appendChild(td);
      });
      tbody.appendChild(tr);
    });
    table.appendChild(tbody);
    wrap.appendChild(table);
    wrap.addEventListener("mousedown", (event) => {
      event.preventDefault();
      view.dispatch({
        selection: { anchor: this.from },
        scrollIntoView: true,
      });
      view.focus();
    });
    return wrap;
  }
  ignoreEvent() {
    return false;
  }
}
