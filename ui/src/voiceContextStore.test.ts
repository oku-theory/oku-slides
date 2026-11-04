import { describe, expect, test } from "bun:test";
import type { AgentUi } from "./api";
import { createVoiceContextStore } from "./voiceContextStore";

const UI: AgentUi = { slide: "s-1", element: null, field: null, editing: false, widget: null };

describe("voiceContextStore", () => {
  test("voiceContextStore write from App is read by the hook", () => {
    const store = createVoiceContextStore();
    let heard = 0;
    store.subscribe(() => heard++);
    store.write("t-1", { ui: UI, hasFocus: true });
    expect(store.read("t-1")).toEqual({ ui: UI, hasFocus: true });
    expect(store.read("t-2")).toBeNull();
    expect(heard).toBe(1);
  });

  test("voiceContextStore same context written again notifies nobody", () => {
    const store = createVoiceContextStore();
    let heard = 0;
    store.write("t-1", { ui: UI, hasFocus: true });
    store.subscribe(() => heard++);
    store.write("t-1", { ui: { ...UI }, hasFocus: true });
    expect(heard).toBe(0);
    store.write("t-1", { ui: { ...UI, element: "e-1" }, hasFocus: true });
    expect(heard).toBe(1);
  });

  test("voiceContextStore numbers reports per page and never twice", () => {
    const [a, b] = [createVoiceContextStore(), createVoiceContextStore()];
    expect(a.page).not.toBe(b.page);
    expect([a.nextSeq(), a.nextSeq(), b.nextSeq()]).toEqual([1, 2, 1]);
  });

  test("voiceContextStore resume flag starts off and follows the user", () => {
    const store = createVoiceContextStore();
    expect(store.resumed()).toBe(false);
    store.setResumed(true);
    expect(store.resumed()).toBe(true);
  });
});
