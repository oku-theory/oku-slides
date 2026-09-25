import { beforeAll, describe, expect, test } from "bun:test";
import { cellLabel } from "./grid";
import { setMessages, t } from "./i18n";

const ftl = (lang: string) =>
  Bun.file(new URL(`../../crates/slider-i18n/src/locales/${lang}/ui.ftl`, import.meta.url)).text();

/** Identifiants des messages d'une ressource Fluent (hors termes et commentaires). */
const keysOf = (source: string) =>
  new Set([...source.matchAll(/^([a-z][a-z0-9-]*) =/gm)].map((m) => m[1]));

/** Identifiants `ui-…` écrits en littéral dans le code de l'interface (tests exclus). */
async function usedIds(): Promise<Map<string, string>> {
  const used = new Map<string, string>();
  for await (const file of new Bun.Glob("**/*.{ts,tsx}").scan(import.meta.dir)) {
    if (/\.test\.tsx?$/.test(file)) continue;
    const code = await Bun.file(`${import.meta.dir}/${file}`).text();
    for (const m of code.matchAll(/"(ui-[a-z0-9-]+)"/g)) used.set(m[1], file);
  }
  return used;
}

let en = "";
let fr = "";

beforeAll(async () => {
  en = await ftl("en");
  fr = await ftl("fr");
});

describe("messages de l'interface", () => {
  test("chaque identifiant utilisé par l'interface existe en anglais", async () => {
    const keys = keysOf(en);
    const used = await usedIds();
    expect(used.size).toBeGreaterThan(50);
    const missing = [...used].filter(([id]) => !keys.has(id)).map(([id, file]) => `${id} (${file})`);
    expect(missing).toEqual([]);
  });

  test("le français a exactement les clés de l'anglais", () => {
    expect([...keysOf(fr)]).toEqual([...keysOf(en)]);
  });

  test("les messages français gardent les textes d'origine, pluriels et nombres compris", () => {
    setMessages("fr", [en, fr]);
    expect(t("ui-caption-slides", { count: 1, status: "enregistré" })).toBe("1 slide · enregistré");
    expect(t("ui-caption-slides", { count: 8, status: "enregistré" })).toBe("8 slides · enregistré");
    expect(t("ui-seconds", { value: 1.5 })).toBe("1,5 s");
    expect(t("ui-toast-select-zone", { text: "Bonjour" })).toBe("Sélectionnez une zone pour y placer : « Bonjour »");
    expect(t("ui-field", { kind: "kicker" })).toBe("Surtitre");
    expect(t("ui-field", { kind: "autre" })).toBe("autre");
    expect(cellLabel({ col: 0, row: 1, cols: 8 })).toBe("col. 1–8 · lig. 2");
  });

  test("les mêmes messages en anglais", () => {
    setMessages("en", [en]);
    expect(t("ui-caption-templates", { count: 1, status: "saved" })).toBe("1 template · saved");
    expect(t("ui-seconds", { value: 1.5 })).toBe("1.5 s");
    expect(t("ui-undo-add", { kind: "card" })).toBe("Add a card");
  });
});
