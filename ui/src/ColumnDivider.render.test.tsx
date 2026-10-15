// Rendu des séparateurs de colonnes dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { Columns } from "./columns";
import type { ColumnWidths } from "./useKeptUi";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: ColumnDivider, ColumnDividers } = await import("./ColumnDivider");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function mount(side: "pile" | "notes") {
  const resized: [Columns, boolean][] = [];
  let resets = 0;
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => {
    root.render(
      createElement(ColumnDivider, {
        side,
        label: "Largeur",
        measure: () => ({ table: 1440, pile: 240, notes: 500 }),
        onResize: (cols: Columns, done: boolean) => resized.push([cols, done]),
        onReset: () => resets++,
      }),
    );
  });
  const el = host.querySelector<HTMLElement>("[role=separator]")!;
  el.setPointerCapture = () => {};
  const pointer = (type: string, clientX: number) =>
    act(async () => {
      el.dispatchEvent(new PointerEvent(type, { bubbles: true, button: 0, pointerId: 1, clientX, clientY: 300 }));
    });
  return { el, resized, resets: () => resets, pointer };
}

describe("séparateurs de colonnes", () => {
  test("ColumnDivider séparateur vertical nommé", async () => {
    const { el } = await mount("pile");
    expect([el.getAttribute("aria-orientation"), el.getAttribute("aria-label")]).toEqual(["vertical", "Largeur"]);
  });

  test("ColumnDivider glisser la pile vers la droite l'élargit, puis valide au relâcher", async () => {
    const { resized, pointer } = await mount("pile");
    await pointer("pointerdown", 240);
    await pointer("pointermove", 280);
    await pointer("pointerup", 280);
    expect(resized.at(-1)).toEqual([{ pile: 280, notes: 500 }, true]);
  });

  test("ColumnDivider glisser le fil vers la droite le rétrécit", async () => {
    const { resized, pointer } = await mount("notes");
    await pointer("pointerdown", 940);
    await pointer("pointermove", 980);
    expect(resized.at(-1)).toEqual([{ pile: 240, notes: 460 }, false]);
  });

  test("ColumnDivider glisser est borné par fitColumns", async () => {
    const { resized, pointer } = await mount("pile");
    await pointer("pointerdown", 240);
    await pointer("pointermove", 900);
    expect(resized.at(-1)?.[0].pile).toBe(360);
  });

  test("ColumnDivider flèche gauche sur le fil l'élargit d'un pas, validé", async () => {
    const { el, resized } = await mount("notes");
    await act(async () => {
      el.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, key: "ArrowLeft" }));
    });
    expect(resized).toEqual([[{ pile: 240, notes: 516 }, true]]);
  });

  test("ColumnDivider un clic sans mouvement ne fige aucune largeur", async () => {
    const { resized, pointer } = await mount("pile");
    await pointer("pointerdown", 240);
    await pointer("pointerup", 240);
    expect(resized).toEqual([]);
  });

  test("ColumnDivider double-clic n'envoie que le retour par défaut, pas ses deux clics", async () => {
    const { el, resized, resets, pointer } = await mount("notes");
    for (let i = 0; i < 2; i++) {
      await pointer("pointerdown", 940);
      await pointer("pointerup", 940);
    }
    await act(async () => {
      el.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
    });
    expect([resized, resets()]).toEqual([[], 1]);
  });

  test("ColumnDivider clic interrompu sans mouvement ne fige aucune largeur", async () => {
    const { resized, pointer } = await mount("pile");
    await pointer("pointerdown", 240);
    await pointer("pointercancel", 240);
    expect(resized).toEqual([]);
  });

  test("ColumnDivider double-clic revient à la largeur par défaut", async () => {
    const { el, resets } = await mount("pile");
    await act(async () => {
      el.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
    });
    expect(resets()).toBe(1);
  });
});

describe("séparateurs posés sur la table", () => {
  async function table(columns: ColumnWidths) {
    const changes: ColumnWidths[] = [];
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    mounted = { root, host };
    const render = (c: ColumnWidths) =>
      act(async () => root.render(createElement(ColumnDividers, { columns: c, onChange: (x: ColumnWidths) => changes.push(x) })));
    await render(columns);
    const dividers = [...host.querySelectorAll<HTMLElement>("[role=separator]")];
    return { host, changes, dividers, render };
  }

  test("ColumnDividers largeurs choisies posées sur la table", async () => {
    const { host } = await table({ pile: 200, notes: 540 });
    expect([host.style.getPropertyValue("--pile-w"), host.style.getPropertyValue("--notes-w")]).toEqual(["200px", "540px"]);
  });

  test("ColumnDividers largeur par défaut : rien posé, la feuille de style décide", async () => {
    const { host, render } = await table({ pile: 200, notes: null });
    await render({ pile: null, notes: null });
    expect([host.style.getPropertyValue("--pile-w"), host.style.getPropertyValue("--notes-w")]).toEqual(["", ""]);
  });

  test("ColumnDividers double-clic sur le séparateur du fil ne rend que le fil par défaut", async () => {
    const { changes, dividers } = await table({ pile: 200, notes: 540 });
    await act(async () => {
      dividers[1].dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
    });
    expect(changes).toEqual([{ pile: 200, notes: null }]);
  });
});
