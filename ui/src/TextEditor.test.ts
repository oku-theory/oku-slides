import { describe, expect, test } from "bun:test";
import { history, undo } from "@codemirror/commands";
import type { Transaction } from "@codemirror/state";
import { EditorState } from "@codemirror/state";
import { singleLine } from "./TextEditor";

describe("singleLine", () => {
  test("collage multiligne dans un champ : les retours à la ligne deviennent des espaces", () => {
    const state = EditorState.create({ doc: "a", extensions: [singleLine] });
    expect(state.update({ changes: { from: 1, insert: "\nb\nc" } }).state.doc.toString()).toBe("a b c");
  });

  test("collage aplati : une seule annulation rend le texte d'avant", () => {
    const start = EditorState.create({ doc: "a", extensions: [history(), singleLine] });
    const pasted = start.update({ changes: { from: 1, insert: "\nb" }, userEvent: "input.paste" }).state;
    let undone: Transaction | undefined;
    undo({ state: pasted, dispatch: (tr) => { undone = tr; } });
    expect(undone?.state.doc.toString()).toBe("a");
  });
});
