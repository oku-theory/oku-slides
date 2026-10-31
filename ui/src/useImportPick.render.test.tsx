// A .pptx given by the OS: the workspace view opens its window's import dialog.

import { afterAll, afterEach, expect, test } from "bun:test";
import "./test/dom";
import { resetMessages } from "./test/messages";
import type { ImportPick, WorkspaceView } from "./api";

// See ./a2ui/catalog.render.test.tsx: React loads after the DOM.
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

const api = { packs: () => Promise.resolve([]) };

/** The view numbered `generation`, with the queued import `picks`. */
function view(generation: number, picks: ImportPick[]): WorkspaceView {
  return { generation, focused: "w-1", agent_detached: false, import_picks: picks, windows: [] };
}

/**
 * A window `label`: the import dialog as the `App` shows it. `begin` starts importing the shown
 * file, `done` ends it.
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
      imports.job,
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
const kitchen: ImportPick = { window: "w-1", path: "/a/Kitchen.pptx", id: 2 };

test("useImportPick opens the import dialog for a pick of this window", async () => {
  const m = await mount("w-1");
  await m.show(view(1, [alto]));
  expect(m.title()).toBe("Importer une présentation PowerPoint");
  expect(m.picks).toEqual(["/a/Alto.pptx"]);
  expect(m.acks).toEqual([1]);
});

test("useImportPick does not reopen the closed dialog for the same pick", async () => {
  const m = await mount("w-1");
  await m.show(view(1, [alto]));
  await m.close();
  await m.show(view(2, [alto]));
  await m.show(view(3, [alto]));
  expect(m.title()).toBeNull();
  expect(m.picks).toEqual(["/a/Alto.pptx"]);
  expect(m.acks).toEqual([1]);
});

test("useImportPick ignores another window's pick", async () => {
  const m = await mount("w-2");
  await m.show(view(1, [alto]));
  expect(m.title()).toBeNull();
  expect(m.picks).toEqual([]);
  expect(m.acks).toEqual([]);
});

test("useImportPick reopens the dialog for a newer pick", async () => {
  const m = await mount("w-1");
  await m.show(view(1, [alto]));
  await m.close();
  await m.show(view(2, [kitchen]));
  expect(m.title()).toBe("Importer une présentation PowerPoint");
  expect(m.picks).toEqual(["/a/Alto.pptx", "/a/Kitchen.pptx"]);
  expect(m.acks).toEqual([1, 2]);
});

test("useImportPick keeps a pick that arrives during an import and opens it once the import ends", async () => {
  const m = await mount("w-1");
  await m.show(view(1, [alto]));
  await m.begin();
  await m.close();
  await m.show(view(2, [kitchen]));
  expect(m.file()).toBe("Alto.pptx");
  expect(m.acks).toEqual([1]);
  await m.done();
  expect(m.file()).toBe("Kitchen.pptx");
  expect(m.acks).toEqual([1, 2]);
});

test("useImportPick offers queued picks one after another, the next once the dialog closes", async () => {
  const m = await mount("w-1");
  await m.show(view(1, [alto, kitchen]));
  expect(m.file()).toBe("Alto.pptx");
  await m.show(view(2, [kitchen]));
  expect(m.file()).toBe("Alto.pptx");
  expect(m.acks).toEqual([1]);
  await m.close();
  expect(m.file()).toBe("Kitchen.pptx");
  expect(m.acks).toEqual([1, 2]);
});

test("useImportPick shows the running import once for a waiting pick, not again after closing", async () => {
  const m = await mount("w-1");
  await m.show(view(1, [alto]));
  await m.begin();
  await m.close();
  await m.show(view(2, [kitchen]));
  expect(m.file()).toBe("Alto.pptx");
  await m.close();
  await m.show(view(3, [kitchen]));
  expect(m.title()).toBeNull();
  await m.done();
  expect(m.file()).toBe("Kitchen.pptx");
  expect(m.acks).toEqual([1, 2]);
});

test("useImportPick offers the lowest id first when the queue is out of order", async () => {
  const m = await mount("w-1");
  await m.show(view(1, [{ ...alto, id: 3 }, kitchen]));
  expect(m.file()).toBe("Kitchen.pptx");
  await m.close();
  expect(m.file()).toBe("Alto.pptx");
  expect(m.acks).toEqual([2, 3]);
});

test("useImportPick in a queue mixing windows takes only this window's picks", async () => {
  const m = await mount("w-1");
  await m.show(view(1, [{ ...alto, window: "w-2" }, kitchen]));
  expect(m.picks).toEqual(["/a/Kitchen.pptx"]);
  expect(m.acks).toEqual([2]);
});
