// Un .pptx reçu de l'OS : la vue de l'espace de travail ouvre le dialogue d'import de sa fenêtre.

import { afterAll, afterEach, expect, test } from "bun:test";
import "./test/dom";
import { resetMessages } from "./test/messages";
import type { ImportPick, WorkspaceView } from "./api";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const ImportDialog = (await import("./ImportDialog")).default;
const { useImportJob } = await import("./useImportJob");
const { useImportPick } = await import("./useImportPick");
const { setMessages } = await import("./i18n");

afterAll(resetMessages);
setMessages("fr", ["ui-import-title = Importer une présentation PowerPoint\n"]);

let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  await cleanup?.();
  cleanup = null;
});

const api = { templateSets: () => Promise.resolve([]) };

/** La vue de numéro `generation`, avec le choix d'import `pick`. */
function view(generation: number, pick: ImportPick | null): WorkspaceView {
  return { generation, focused: "w-1", agent_detached: false, import_pick: pick, windows: [] };
}

/**
 * Une fenêtre `label` : le dialogue d'import tel que l'`App` le montre. `begin` lance l'import du
 * fichier montré, `done` le termine.
 */
async function mount(label: string) {
  const state = {
    close: () => {},
    begin: () => {},
    finish: () => {},
    picks: [] as string[],
    acks: [] as number[],
  };
  const Probe = ({ v }: { v: WorkspaceView | null }) => {
    const imports = useImportJob(() => {});
    state.close = imports.close;
    state.begin = () =>
      void imports.start({ target: { mode: "hifi" }, tune: false }, () => new Promise((ok) => (state.finish = ok)));
    useImportPick(
      v,
      label,
      imports.job?.busy ?? false,
      (path) => {
        state.picks.push(path);
        return imports.pick(path);
      },
      (id) => void state.acks.push(id),
    );
    const job = imports.job?.shown ? imports.job : null;
    return job
      ? createElement(ImportDialog, {
          path: job.path,
          api,
          themes: [],
          busy: job.busy,
          error: null,
          agentUnusable: null,
          onImport: () => {},
          onClose: imports.close,
        })
      : null;
  };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  const show = (v: WorkspaceView | null) => act(async () => root.render(createElement(Probe, { v })));
  await show(null);
  cleanup = async () => {
    await act(async () => root.unmount());
    host.remove();
  };
  const title = () => host.querySelector("h2")?.textContent ?? null;
  const file = () => host.querySelector("#import-file")?.textContent ?? null;
  return {
    show,
    title,
    file,
    picks: state.picks,
    acks: state.acks,
    close: () => act(async () => state.close()),
    begin: () => act(async () => state.begin()),
    done: () => act(async () => state.finish()),
  };
}

const alto: ImportPick = { window: "w-1", path: "/a/Alto.pptx", id: 1 };

test("useImportPick ouvre le dialogue d'import pour un choix de cette fenêtre", async () => {
  const m = await mount("w-1");
  await m.show(view(1, alto));
  expect(m.title()).toBe("Importer une présentation PowerPoint");
  expect(m.picks).toEqual(["/a/Alto.pptx"]);
  expect(m.acks).toEqual([1]);
});

test("useImportPick ne rouvre pas le dialogue fermé pour le même choix", async () => {
  const m = await mount("w-1");
  await m.show(view(1, alto));
  await m.close();
  await m.show(view(2, alto));
  await m.show(view(3, alto));
  expect(m.title()).toBeNull();
  expect(m.picks).toEqual(["/a/Alto.pptx"]);
  expect(m.acks).toEqual([1]);
});

test("useImportPick ignore le choix d'une autre fenêtre", async () => {
  const m = await mount("w-2");
  await m.show(view(1, alto));
  expect(m.title()).toBeNull();
  expect(m.picks).toEqual([]);
  expect(m.acks).toEqual([]);
});

test("useImportPick rouvre le dialogue pour un choix plus récent", async () => {
  const m = await mount("w-1");
  await m.show(view(1, alto));
  await m.close();
  await m.show(view(2, { window: "w-1", path: "/a/Kitchen.pptx", id: 2 }));
  expect(m.title()).toBe("Importer une présentation PowerPoint");
  expect(m.picks).toEqual(["/a/Alto.pptx", "/a/Kitchen.pptx"]);
  expect(m.acks).toEqual([1, 2]);
});

test("useImportPick garde un choix arrivé pendant un import et l'ouvre une fois l'import fini", async () => {
  const m = await mount("w-1");
  await m.show(view(1, alto));
  await m.begin();
  await m.close();
  await m.show(view(2, { window: "w-1", path: "/a/Kitchen.pptx", id: 2 }));
  expect(m.file()).toBe("Alto.pptx");
  expect(m.acks).toEqual([1]);
  await m.done();
  expect(m.file()).toBe("Kitchen.pptx");
  expect(m.acks).toEqual([1, 2]);
});
