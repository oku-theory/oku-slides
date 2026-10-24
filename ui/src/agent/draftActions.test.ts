import { expect, test } from "bun:test";
import { draftActions } from "./draftActions";

function deps(over: Partial<Parameters<typeof draftActions>[0]> = {}) {
  const log: string[] = [];
  const base = {
    draft: { append: (t: string) => void log.push(`append:${t}`), close: () => void log.push("close") },
    editing: false,
    insert: (t: string) => (log.push(`insert:${t}`), true),
    prompt: async (t: string, images: string[]) => (log.push(["prompt", t, ...images].join(":")), true),
  };
  return { log, actions: draftActions({ ...base, ...over }) };
}

test("draftActions dictée pendant l'édition : au curseur", () => {
  const { log, actions } = deps({ editing: true });
  actions.onDictated("mot", "auto");
  expect(log).toEqual(["insert:mot"]);
});

test("draftActions dictée hors édition : au brouillon", () => {
  const { log, actions } = deps();
  actions.onDictated("mot", "auto");
  expect(log).toEqual(["append:mot"]);
});

test("draftActions envoyer : ferme le brouillon une fois la consigne partie", async () => {
  const { log, actions } = deps();
  await actions.sendDraft("fais ceci", []);
  expect(log).toEqual(["prompt:fais ceci", "close"]);
});

test("draftActions envoyer en échec : le brouillon reste", async () => {
  const { log, actions } = deps({ prompt: async () => false });
  await actions.sendDraft("fais ceci", []);
  expect(log).toEqual([]);
});

test("draftActions dictée au curseur refusée : le texte revient au brouillon", async () => {
  const { log, actions } = deps({ editing: true, insert: () => false });
  actions.onDictated("mot", "auto");
  await new Promise((r) => setTimeout(r));
  expect(log).toEqual(["append:mot"]);
});

test("draftActions envoyer passe les images à la consigne", async () => {
  const { log, actions } = deps();
  await actions.sendDraft("fais ceci", ["sha256:aa"]);
  expect(log).toEqual(["prompt:fais ceci:sha256:aa", "close"]);
});
