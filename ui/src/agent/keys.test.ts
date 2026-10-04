import { describe, expect, test } from "bun:test";
import "../test/dom";
import { dictationShortcut, escapeAction, targetOwnsKeys, typingShortcut } from "./keys";

const k = (
  key: string,
  mods: { meta?: boolean; ctrl?: boolean; shift?: boolean; alt?: boolean; repeat?: boolean; code?: string } = {},
) => ({
  key,
  code: mods.code ?? `Key${key.toUpperCase()}`,
  metaKey: !!mods.meta,
  ctrlKey: !!mods.ctrl,
  shiftKey: !!mods.shift,
  altKey: !!mods.alt,
  repeat: !!mods.repeat,
});
const idle = { agentBusy: false, recording: false };
const busy = { agentBusy: true, recording: false };

describe("raccourcis de la dictée", () => {
  test("⌘D dicte là où l'on écrit", () => expect(dictationShortcut(k("d", { meta: true }), idle)).toBe("auto"));
  test("Ctrl+D aussi", () => expect(dictationShortcut(k("d", { ctrl: true }), idle)).toBe("auto"));
  test("⌘⇧D dicte dans le brouillon", () => expect(dictationShortcut(k("D", { meta: true, shift: true }), idle)).toBe("draft"));
  test("Ctrl⇧D aussi", () => expect(dictationShortcut(k("d", { ctrl: true, shift: true }), idle)).toBe("draft"));
  test("⌘⇧D pendant un tour arrête l'agent", () =>
    expect(dictationShortcut(k("d", { meta: true, shift: true }), busy)).toBe("agent-cancel"));
  test("⌘D pendant un tour dicte toujours", () =>
    expect(dictationShortcut(k("d", { meta: true }), busy)).toBe("auto"));
  test("pendant une écoute, ⌘⇧D l'arrête au lieu d'arrêter l'agent", () =>
    expect(dictationShortcut(k("d", { meta: true, shift: true }), { agentBusy: true, recording: true })).toBe("draft"));
  test("une touche maintenue ne répète pas le raccourci", () =>
    expect(dictationShortcut(k("d", { meta: true, repeat: true }), idle)).toBeNull());
  test("⌘⌥D n'est pas un raccourci de dictée", () =>
    expect(dictationShortcut(k("d", { meta: true, alt: true }), idle)).toBeNull());
  test("sur un clavier cyrillique, ⌘D est lu par sa position", () =>
    expect(dictationShortcut(k("в", { meta: true, code: "KeyD" }), idle)).toBe("auto"));
  test("D seul, ⇧D ou ⌘S ne font rien", () => {
    expect(dictationShortcut(k("d"), idle)).toBeNull();
    expect(dictationShortcut(k("D", { shift: true }), idle)).toBeNull();
    expect(dictationShortcut(k("s", { meta: true }), idle)).toBeNull();
  });
});

describe("Échap", () => {
  test("désélectionne d'abord", () => expect(escapeAction({ selected: true, agentBusy: true })).toBe("deselect"));
  test("arrête l'agent quand rien n'est sélectionné", () =>
    expect(escapeAction({ selected: false, agentBusy: true })).toBe("agent-cancel"));
  test("ne fait rien sinon", () => expect(escapeAction({ selected: false, agentBusy: false })).toBeNull());
});

describe("touches d'une cible", () => {
  const none = { metaKey: false, ctrlKey: false };
  const mod = { metaKey: true, ctrlKey: false };
  const inDom = (html: string) => {
    const host = document.createElement("div");
    host.innerHTML = html;
    return host.querySelector("[data-t]") as HTMLElement;
  };
  test("un champ de texte d'une surface garde ⌘Z", () => {
    const input = inDom('<div class="a2ui-surface"><input data-t></div>');
    expect(targetOwnsKeys(input, mod)).toBe(true);
    expect(targetOwnsKeys(input, none)).toBe(true);
  });
  test("un bouton du dock laisse passer ⌘Z mais garde les touches seules", () => {
    const button = inDom('<section class="agent-dock agent-keys"><div class="a2ui-surface"><button data-t>+</button></div></section>');
    expect(targetOwnsKeys(button, mod)).toBe(false);
    expect(targetOwnsKeys(button, { metaKey: false, ctrlKey: true })).toBe(false);
    expect(targetOwnsKeys(button, none)).toBe(true);
  });
  test("hors surface, un bouton ne retient rien", () => {
    const button = inDom("<div><button data-t>ok</button></div>");
    expect(targetOwnsKeys(button, none)).toBe(false);
    expect(targetOwnsKeys(null, none)).toBe(false);
  });
});

describe("écrire à l'agent", () => {
  test("⌘K ouvre le brouillon", () => expect(typingShortcut(k("k", { meta: true }))).toBe(true));
  test("Ctrl+K aussi", () => expect(typingShortcut(k("k", { ctrl: true }))).toBe(true));
  test("sur un clavier cyrillique, ⌘K est lu par sa position", () =>
    expect(typingShortcut(k("л", { meta: true, code: "KeyK" }))).toBe(true));
  test("K seul, ⌘⇧K, ⌘⌥K ou une touche maintenue ne font rien", () => {
    expect(typingShortcut(k("k"))).toBe(false);
    expect(typingShortcut(k("K", { meta: true, shift: true }))).toBe(false);
    expect(typingShortcut(k("k", { meta: true, alt: true }))).toBe(false);
    expect(typingShortcut(k("k", { meta: true, repeat: true }))).toBe(false);
  });
});
