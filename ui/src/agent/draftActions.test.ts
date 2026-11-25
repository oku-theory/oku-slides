import { expect, test } from "bun:test";
import type { PastedImage } from "../api";
import { afterSend, appendDraft } from "../draft";
import { draftActions, promptThrough } from "./draftActions";

function deps(over: Partial<Parameters<typeof draftActions>[0]> = {}, images: PastedImage[] = [], before: string | null = null) {
  const log: string[] = [];
  let text = before;
  const base = {
    draft: {
      append: (t: string) => (log.push(`append:${t}`), (text = appendDraft(text, t)), { text, images }),
      drop: (t: string) => {
        log.push(`drop:${t}`);
        text = afterSend(text, t);
        return text !== null;
      },
    },
    removeImage: (id: string) => void log.push(`remove:${id}`),
    flight: { current: false },
    editing: false,
    insert: (t: string) => (log.push(`insert:${t}`), true),
    prompt: async (t: string, images: string[]) => (log.push(["prompt", t, ...images].join(":")), true),
  };
  return { log, actions: draftActions({ ...base, ...over }), text: () => text };
}

test("draftActions dictée pendant l'édition : au curseur", () => {
  const { log, actions } = deps({ editing: true });
  actions.onDictated("mot", "auto", false);
  expect(log).toEqual(["insert:mot"]);
});

test("draftActions dictée hors édition : au brouillon", () => {
  const { log, actions } = deps();
  actions.onDictated("mot", "auto", false);
  expect(log).toEqual(["append:mot"]);
});

test("draftActions envoyer : ferme le brouillon une fois la consigne partie", async () => {
  const { log, actions } = deps();
  await actions.sendDraft("fais ceci", []);
  expect(log).toEqual(["prompt:fais ceci", "drop:fais ceci"]);
});

test("draftActions envoyer en échec : le brouillon reste", async () => {
  const { log, actions } = deps({ prompt: async () => false });
  await actions.sendDraft("fais ceci", []);
  expect(log).toEqual([]);
});

test("draftActions dictée au curseur refusée : le texte revient au brouillon", async () => {
  const { log, actions } = deps({ editing: true, insert: () => false });
  actions.onDictated("mot", "auto", false);
  await new Promise((r) => setTimeout(r));
  expect(log).toEqual(["append:mot"]);
});

test("draftActions envoyer passe les images à la consigne", async () => {
  const { log, actions } = deps();
  await actions.sendDraft("fais ceci", ["sha256:aa"]);
  expect(log).toEqual(["prompt:fais ceci:sha256:aa", "drop:fais ceci"]);
});

const IMAGE: PastedImage = { id: "sha256:aa", mime: "image/png", width: 1, height: 1 };

test("draftActions dictation finished by hand outside editing sends the whole draft then closes it", async () => {
  const { log, actions } = deps({}, [IMAGE], "avant ");
  actions.onDictated("mot", "auto", true);
  await new Promise((r) => setTimeout(r));
  expect(log).toEqual(["append:mot", "prompt:avant mot:sha256:aa", "drop:avant mot"]);
});

test("draftActions dictation finished by hand while editing goes to the cursor, unsent", async () => {
  const { log, actions } = deps({ editing: true });
  actions.onDictated("mot", "auto", true);
  await new Promise((r) => setTimeout(r));
  expect(log).toEqual(["insert:mot"]);
});

test("draftActions dictation sent but refused keeps the draft open", async () => {
  const { log, actions } = deps({ prompt: async () => false });
  actions.onDictated("mot", "auto", true);
  await new Promise((r) => setTimeout(r));
  expect(log).toEqual(["append:mot"]);
});

test("draftActions dictation finished during a send stays in the draft, unsent", async () => {
  let release: (ok: boolean) => void = () => {};
  const prompts: string[] = [];
  const prompt = (t: string) => (prompts.push(t), new Promise<boolean>((r) => (release = r)));
  const { log, actions } = deps({ prompt });
  actions.onDictated("un", "auto", true);
  actions.onDictated("deux", "auto", true);
  release(true);
  await new Promise((r) => setTimeout(r));
  expect(prompts).toEqual(["un"]);
  expect(log).toEqual(["append:un", "append:deux", "drop:un"]);
});

test("draftActions send allowed again once the previous one is refused", async () => {
  let ok = false;
  const { log, actions } = deps({ prompt: async (t) => (log.push(`prompt:${t}`), ok) });
  await actions.sendDraft("un", []);
  ok = true;
  await actions.sendDraft("un", []);
  expect(log).toEqual(["prompt:un", "prompt:un", "drop:un"]);
});

test("promptThrough without a mounted panel reports it and sends nothing", async () => {
  const errors: string[] = [];
  const ok = await promptThrough({ current: null }, (m) => errors.push(m))("fais ceci", []);
  expect(ok).toBe(false);
  expect(errors).toEqual(["ui-agent-panel-missing"]);
});

test("promptThrough with a panel hands the prompt over", async () => {
  const sent: string[] = [];
  const handle = { current: { prompt: async (t: string, i: string[]) => (sent.push([t, ...i].join(":")), true) } };
  expect(await promptThrough(handle, () => {})("fais ceci", ["sha256:aa"])).toBe(true);
  expect(sent).toEqual(["fais ceci:sha256:aa"]);
});

test("draftActions dictation during a send keeps it and drops only the sent images", async () => {
  let accept = (_: boolean) => {};
  const prompt = (t: string, ids: string[]) =>
    new Promise<boolean>((r) => {
      log.push(["prompt", t, ...ids].join(":"));
      accept = r;
    });
  const { log, actions, text } = deps({ prompt }, [IMAGE]);
  actions.onDictated("un", "auto", true);
  actions.onDictated("deux", "auto", true);
  accept(true);
  await Promise.resolve();
  await Promise.resolve();
  expect(log).toEqual(["append:un", "prompt:un:sha256:aa", "append:deux", "drop:un", "remove:sha256:aa"]);
  expect(text()).toBe("deux");
});
