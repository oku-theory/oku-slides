// L'import d'un .pptx tenu par l'espace de travail : le dialogue peut se fermer, l'issue reste montrée.

import { afterEach, expect, test } from "bun:test";
import "./test/dom";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useImportJob } = await import("./useImportJob");

type Job = ReturnType<typeof useImportJob>;
let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  await cleanup?.();
  cleanup = null;
});

/** Monte le hook ; `settle` règle l'import en cours (rien : réussi, un texte : refusé). */
async function mount() {
  const late: string[] = [];
  const started: string[] = [];
  let settle: (error?: string) => void = () => {};
  const run = (path: string) =>
    new Promise<void>((resolve, reject) => {
      started.push(path);
      settle = (error) => (error === undefined ? resolve() : reject(new Error(error)));
    });
  const state: { job: Job | null } = { job: null };
  const Probe = () => {
    state.job = useImportJob((m) => late.push(m));
    return null;
  };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(createElement(Probe)));
  cleanup = async () => {
    await act(async () => root.unmount());
    host.remove();
  };
  const job = () => state.job!;
  const done = (error?: string) => act(async () => settle(error));
  const begin = () => act(async () => void job().start({ target: { mode: "hifi" }, tune: true }, run));
  return { job, late, started, done, begin };
}

test("useImportJob pick ouvre le dialogue et retient les touches des onglets", async () => {
  const { job } = await mount();
  expect(job().job).toBeNull();
  await act(async () => job().pick("/a/Alto.pptx"));
  expect(job().job).toEqual({ path: "/a/Alto.pptx", shown: true, busy: false, error: null });
  expect(job().holdsKeys()).toBe(true);
});

test("useImportJob réussi oublie l'import", async () => {
  const { job, started, done, begin } = await mount();
  await act(async () => job().pick("/a/Alto.pptx"));
  await begin();
  expect(started).toEqual(["/a/Alto.pptx"]);
  expect(job().job?.busy).toBe(true);
  await done();
  expect(job().job).toBeNull();
});

test("useImportJob refusé, dialogue ouvert : le refus y reste", async () => {
  const { job, late, done, begin } = await mount();
  await act(async () => job().pick("/a/Alto.pptx"));
  await begin();
  await done("fichier illisible");
  expect(job().job).toEqual({ path: "/a/Alto.pptx", choice: { target: { mode: "hifi" }, tune: true }, shown: true, busy: false, error: "fichier illisible" });
  expect(late).toEqual([]);
});

test("useImportJob fermé pendant l'import : il continue, sans retenir les touches", async () => {
  const { job, done, begin } = await mount();
  await act(async () => job().pick("/a/Alto.pptx"));
  await begin();
  await act(async () => job().close());
  expect(job().job).toEqual({ path: "/a/Alto.pptx", choice: { target: { mode: "hifi" }, tune: true }, shown: false, busy: true, error: null });
  expect(job().holdsKeys()).toBe(false);
  await done();
  expect(job().job).toBeNull();
});

test("useImportJob refusé après la fermeture du dialogue : l'espace de travail le montre", async () => {
  const { job, late, done, begin } = await mount();
  await act(async () => job().pick("/a/Alto.pptx"));
  await begin();
  await act(async () => job().close());
  await done("fichier illisible");
  expect(job().job).toBeNull();
  expect(late).toEqual(["fichier illisible"]);
});

test("useImportJob un autre choix pendant l'import rouvre celui qui tourne et se dit refusé", async () => {
  const { job, started, begin } = await mount();
  const taken: boolean[] = [];
  await act(async () => void taken.push(job().pick("/a/Alto.pptx")));
  await begin();
  await act(async () => job().close());
  await act(async () => void taken.push(job().pick("/b/Autre.pptx")));
  expect(taken).toEqual([true, false]);
  await begin();
  expect(job().job).toEqual({ path: "/a/Alto.pptx", choice: { target: { mode: "hifi" }, tune: true }, shown: true, busy: true, error: null });
  expect(started).toHaveLength(1);
});

test("useImportJob fermé hors import oublie le refus", async () => {
  const { job, done, begin } = await mount();
  await act(async () => job().pick("/a/Alto.pptx"));
  await begin();
  await done("fichier illisible");
  await act(async () => job().close());
  expect(job().job).toBeNull();
});
