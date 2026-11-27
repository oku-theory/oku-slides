// Rendu du brouillon de consigne dans un DOM happy-dom.

import { afterAll, afterEach, beforeAll, describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import "./test/dom";
import type { Attachment } from "./api";
import { applyMessages } from "./i18n";
import { resetMessages } from "./test/messages";

// Le mot-raccourci vient de `ui-shortcut-theme` : le français est chargé pour ce fichier.
const ftl = (l: string) => readFileSync(`${import.meta.dir}/../../crates/oku-slides-i18n/src/locales/${l}/ui.ftl`, "utf8");
beforeAll(() => applyMessages({ locale: "fr", resources: [ftl("en"), ftl("fr")], chosen: null }));
afterAll(resetMessages);

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { DictationDraft } = await import("./DictationDraft");
const { expectIconOnly } = await import("./test/iconOnly");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function mount(
  text: string | null,
  {
    busy = false,
    typing = 0,
    ready = true,
    shortcuts = false,
    attachments = [] as Attachment[],
    thumbs = new Map<string, string>(),
  } = {},
) {
  const sent: string[] = [];
  const sentImages: string[][] = [];
  const pastes: File[][] = [];
  const removed: string[] = [];
  const changes: string[] = [];
  let discarded = 0;
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => {
    root.render(
      createElement(DictationDraft, {
        text,
        dictations: 1,
        typing,
        busy,
        ready,
        shortcuts,
        onChange: (v: string) => changes.push(v),
        attachments,
        thumbs,
        onAttachFiles: (f: File[]) => pastes.push(f),
        onRemoveAttachment: (id: string) => removed.push(id),
        onSend: (v: string, ids: string[]) => (sent.push(v), sentImages.push(ids)),
        onDiscard: () => discarded++,
      }),
    );
  });
  const area = () => host.querySelector("textarea") as HTMLTextAreaElement;
  const note = () => host.querySelector(".draft-note") as HTMLElement;
  const send = () => host.querySelector(".draft-send") as HTMLButtonElement;
  return { host, area, note, send, sent, sentImages, pastes, removed, changes, discarded: () => discarded };
}

const key = (el: Element, init: KeyboardEventInit) =>
  el.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...init }));

describe("brouillon de dictée", () => {
  test("DictationDraft sends the shortcut word with no agent ready", async () => {
    const d = await mount("Thème.", { ready: false, shortcuts: true });
    expect(d.send().disabled).toBe(false);
  });

  test("DictationDraft sends the shortcut word during a turn", async () => {
    const d = await mount("thème", { busy: true, shortcuts: true });
    expect(d.send().disabled).toBe(false);
  });

  test("DictationDraft keeps the shortcut word gated during a turn outside deck mode", async () => {
    const d = await mount("thème", { busy: true, shortcuts: false });
    expect(d.send().disabled).toBe(true);
  });

  test("DictationDraft keeps other text gated during a turn", async () => {
    const d = await mount("change le thème", { busy: true });
    expect(d.send().disabled).toBe(true);
  });

  test("DictationDraft sans agent disponible ne peut pas envoyer (t-211)", async () => {
    const d = await mount("Ajoute une slide", { ready: false });
    expect(d.send().disabled).toBe(true);
    key(d.note(), { key: "Enter", metaKey: true });
    expect(d.sent).toEqual([]);
  });

  test("DictationDraft sans brouillon ne montre rien", async () => {
    const { host } = await mount(null);
    expect(host.querySelector(".draft")).toBeNull();
  });

  test("DictationDraft actions show icons only", async () => {
    const { host, send } = await mount("plus court");
    expectIconOnly(send());
    expectIconOnly(host.querySelector(".draft-discard"));
  });

  test("le brouillon est une carte de consigne, sans post-it", async () => {
    const { host } = await mount("plus court");
    expect(host.querySelector("section.draft.card.prompt")).not.toBeNull();
    expect(host.querySelector(".postit")).toBeNull();
  });

  test("DictationDraft avec une dictée la montre sur une carte, sans champ", async () => {
    const { note, area } = await mount("rends ce titre plus court");
    expect(note().textContent).toBe("rends ce titre plus court");
    expect(area()).toBeNull();
  });

  test("DictationDraft clic sur la carte ouvre l'édition, curseur en fin", async () => {
    const { note, area } = await mount("plus court");
    await act(async () => note().click());
    expect(area().value).toBe("plus court");
    expect(document.activeElement).toBe(area());
    expect(area().selectionStart).toBe("plus court".length);
  });

  test("DictationDraft Entrée sur la carte ouvre l'édition", async () => {
    const { note, area } = await mount("plus court");
    await act(async () => key(note(), { key: "Enter" }));
    expect(area()).not.toBeNull();
  });

  test("DictationDraft Échap dans le champ revient à la carte", async () => {
    const { note, area } = await mount("plus court");
    await act(async () => note().click());
    await act(async () => key(area(), { key: "Escape" }));
    expect(area()).toBeNull();
    expect(document.activeElement).toBe(note());
  });

  test("DictationDraft retouché transmet le texte", async () => {
    const { note, area, changes } = await mount("plus court");
    await act(async () => note().click());
    await act(async () => {
      const set = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")!.set!;
      set.call(area(), "plus long");
      area().dispatchEvent(new Event("input", { bubbles: true }));
    });
    expect(changes).toEqual(["plus long"]);
  });

  test("DictationDraft n'envoie rien tant qu'on n'appuie pas sur Envoyer", async () => {
    const { sent } = await mount("rends ce titre");
    await act(async () => new Promise((r) => setTimeout(r, 20)));
    expect(sent).toEqual([]);
  });

  test("DictationDraft Envoyer a le style des boutons de l'app", async () => {
    const { send } = await mount("plus court");
    expect(send().classList.contains("object")).toBe(true);
  });

  test("DictationDraft Envoyer envoie le texte sans ses blancs de bord", async () => {
    const { send, sent } = await mount("  plus court \n");
    await act(async () => send().click());
    expect(sent).toEqual(["plus court"]);
  });

  test("DictationDraft ⌘↩ sur la carte envoie", async () => {
    const { note, sent } = await mount("plus court");
    await act(async () => key(note(), { key: "Enter", metaKey: true }));
    expect(sent).toEqual(["plus court"]);
  });

  test("DictationDraft ⌘↩ dans le champ envoie", async () => {
    const { note, area, sent } = await mount("plus court");
    await act(async () => note().click());
    await act(async () => key(area(), { key: "Enter", metaKey: true }));
    expect(sent).toEqual(["plus court"]);
  });

  test("DictationDraft ↩ seul dans le champ n'envoie pas", async () => {
    const { note, area, sent } = await mount("plus court");
    await act(async () => note().click());
    await act(async () => key(area(), { key: "Enter" }));
    expect(sent).toEqual([]);
  });

  test("DictationDraft vide ne peut pas être envoyé", async () => {
    const { note, send, sent } = await mount("   ");
    expect(send().disabled).toBe(true);
    await act(async () => key(note(), { key: "Enter", metaKey: true }));
    expect(sent).toEqual([]);
  });

  test("DictationDraft pendant un tour de l'agent attend pour envoyer", async () => {
    const { note, send, sent } = await mount("plus court", { busy: true });
    expect(send().disabled).toBe(true);
    await act(async () => key(note(), { key: "Enter", metaKey: true }));
    expect(sent).toEqual([]);
  });

  test("DictationDraft Effacer abandonne le brouillon", async () => {
    const { host, discarded } = await mount("plus court");
    await act(async () => (host.querySelector(".draft-discard") as HTMLButtonElement).click());
    expect(discarded()).toBe(1);
  });

  test("DictationDraft après une dictée donne le focus à la carte", async () => {
    const { note } = await mount("plus court");
    expect(document.activeElement).toBe(note());
  });

  test("DictationDraft ⌘⇧↩ n'envoie pas et ne sort pas du brouillon", async () => {
    const { note, sent } = await mount("Budget 2027");
    const outside: KeyboardEvent[] = [];
    const spy = (e: KeyboardEvent) => void outside.push(e);
    window.addEventListener("keydown", spy);
    await act(async () => key(note(), { key: "Enter", metaKey: true, shiftKey: true }));
    window.removeEventListener("keydown", spy);
    expect([sent, outside]).toEqual([[], []]);
  });

  test("DictationDraft ouvert pour écrire est un champ qui a le focus", async () => {
    const { area, note } = await mount("", { typing: 1 });
    expect(note()).toBeNull();
    expect(document.activeElement).toBe(area());
  });

  test("DictationDraft ouvert pour écrire après une dictée met le curseur en fin", async () => {
    const { area } = await mount("plus court", { typing: 1 });
    expect(document.activeElement).toBe(area());
    expect(area().selectionStart).toBe("plus court".length);
  });

  test("DictationDraft vide quitté se referme", async () => {
    const { area, discarded } = await mount("", { typing: 1 });
    await act(async () => area().blur());
    expect(discarded()).toBe(1);
  });

  test("DictationDraft vide, Échap le referme", async () => {
    const { area, discarded } = await mount("  ", { typing: 1 });
    await act(async () => key(area(), { key: "Escape" }));
    expect(discarded()).toBe(1);
  });

  test("DictationDraft écrit puis quitté reste ouvert", async () => {
    const { area, note, discarded } = await mount("plus court", { typing: 1 });
    await act(async () => area().blur());
    expect(discarded()).toBe(0);
    expect(note().textContent).toBe("plus court");
  });

  const png = () => new File([new Uint8Array([137, 80])], "x.png", { type: "image/png" });
  const paste = (text: string) => {
    const e = new Event("paste", { bubbles: true, cancelable: true });
    Object.defineProperty(e, "clipboardData", { value: { getData: () => text, files: [png()] } });
    return e;
  };
  const one: Attachment[] = [{ type: "image", id: "sha256:aa", mime: "image/png", width: 4, height: 3 }];

  test("coller une image appelle onAttachFiles, sans coller de texte", async () => {
    const m = await mount("Pose-la");
    const e = paste("");
    await act(async () => m.note().dispatchEvent(e));
    expect([m.pastes.length, e.defaultPrevented]).toEqual([1, true]);
  });

  test("coller du texte ne joint aucune image", async () => {
    const m = await mount("Pose-la");
    await act(async () => m.note().dispatchEvent(paste("du texte")));
    expect(m.pastes.length).toBe(0);
  });

  const pdf = (name = "plan.pdf") => new File([new Uint8Array([37, 80, 68, 70])], name, { type: "application/pdf" });
  const drop = (files: File[]) => {
    const e = new Event("drop", { bubbles: true, cancelable: true });
    Object.defineProperty(e, "dataTransfer", { value: { types: ["Files"], files } });
    return e;
  };
  const document_ = (name: string): Attachment => ({ type: "document", id: "sha256:dd", name, kind: "pdf", bytes: 2048 });

  test("DictationDraft drop pdf calls onAttachFiles", async () => {
    const m = await mount("Lis-le");
    const e = drop([pdf()]);
    await act(async () => m.note().dispatchEvent(e));
    expect([m.pastes.map((f) => f.map((x) => x.name)), e.defaultPrevented]).toEqual([[["plan.pdf"]], true]);
  });

  test("DictationDraft paste text file keeps text in draft", async () => {
    const m = await mount("Lis-le");
    const e = new Event("paste", { bubbles: true, cancelable: true });
    Object.defineProperty(e, "clipboardData", { value: { getData: () => "un paragraphe copié", files: [pdf()] } });
    await act(async () => m.note().dispatchEvent(e));
    expect([m.pastes.length, e.defaultPrevented]).toEqual([0, false]);
  });

  test("DictationDraft paste finder file copy calls onAttachFiles", async () => {
    const m = await mount("Lis-le");
    const e = new Event("paste", { bubbles: true, cancelable: true });
    // A Finder copy puts the file's name in the text with the file itself.
    Object.defineProperty(e, "clipboardData", { value: { getData: () => "plan.pdf", files: [pdf()] } });
    await act(async () => m.note().dispatchEvent(e));
    expect([m.pastes.length, e.defaultPrevented]).toEqual([1, true]);
  });

  test("DictationDraft document chip shows name and size and removes", async () => {
    const m = await mount("Lis-le", { attachments: [document_("plan.pdf")] });
    const chip = m.host.querySelector(".draft-document");
    expect(chip?.querySelector(".draft-name")?.textContent).toBe("\u2068plan.pdf\u2069");
    expect(chip?.textContent).toContain("2 Ko");
    const remove = chip?.querySelector("button") as HTMLButtonElement;
    expect(remove.getAttribute("aria-label")).toBe("Retirer \u2068plan.pdf\u2069");
    await act(async () => remove.click());
    expect(m.removed).toEqual(["sha256:dd"]);
  });

  test("DictationDraft document chip of 2 MB shows its size in megabytes", async () => {
    const m = await mount("Lis-le", { attachments: [{ ...document_("gros.pdf"), bytes: 2 * 1024 * 1024 }] });
    expect(m.host.querySelector(".draft-document .draft-size")?.textContent).toBe("2 Mo");
  });

  test("DictationDraft document chip title drops hidden controls", async () => {
    const m = await mount("Lis-le", { attachments: [document_("evil\u202Efdp.exe")] });
    expect(m.host.querySelector(".draft-name")?.getAttribute("title")).toBe("evilfdp.exe");
  });

  test("DictationDraft picker button attaches the chosen files", async () => {
    const m = await mount("Lis-le");
    const button = m.host.querySelector(".draft-pick") as HTMLButtonElement;
    const input = m.host.querySelector("input[type=file]") as HTMLInputElement;
    let opened = 0;
    input.click = () => void opened++;
    expectIconOnly(button);
    expect(button.getAttribute("aria-label")).toBe("Joindre des fichiers");
    await act(async () => button.click());
    Object.defineProperty(input, "files", { value: [pdf(), png()], configurable: true });
    await act(async () => input.dispatchEvent(new Event("change", { bubbles: true })));
    expect([opened, input.multiple, m.pastes.map((f) => f.map((x) => x.name))]).toEqual([1, true, [["plan.pdf", "x.png"]]]);
  });

  test("DictationDraft chip name renders markup literally", async () => {
    const m = await mount("Lis-le", { attachments: [document_("<img src=x onerror=alert(1)>.pdf")] });
    const chip = m.host.querySelector(".draft-document");
    expect([chip?.querySelector("img"), chip?.querySelector(".draft-name")?.textContent]).toEqual([
      null,
      "\u2068<img src=x onerror=alert(1)>.pdf\u2069",
    ]);
  });

  test("une puce par image, son bouton la retire", async () => {
    const m = await mount("Pose-la", { attachments: one });
    expect(m.host.querySelector(".draft-image")?.textContent).toContain("\u2068PNG\u2069 \u20684×3\u2069");
    await act(async () => (m.host.querySelector(".draft-image button") as HTMLButtonElement).click());
    expect(m.removed).toEqual(["sha256:aa"]);
  });

  test("la miniature locale s'affiche quand la vue a l'image", async () => {
    const m = await mount("x", { attachments: one, thumbs: new Map([["sha256:aa", "blob:t"]]) });
    expect(m.host.querySelector(".draft-image img")?.getAttribute("src")).toBe("blob:t");
  });

  test("envoyer passe le texte et les ids des images", async () => {
    const m = await mount("Pose-la", { attachments: one });
    await act(async () => m.send().click());
    expect([m.sent, m.sentImages]).toEqual([["Pose-la"], [["sha256:aa"]]]);
  });

  test("des images sans texte : l'envoi attend le texte, et le dit", async () => {
    const m = await mount("", { attachments: one });
    expect([m.send().disabled, m.send().title]).toEqual([true, "Dites ce qu'il faut en faire"]);
  });
});
