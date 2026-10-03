import { describe, expect, test } from "bun:test";
import { NO_TABS, activated, activeName, closed, kept, opened, pruned, renamed, withRestored } from "./widgetTabs";

const names = (s: { tabs: { name: string }[] }) => s.tabs.map((t) => t.name);

describe("widgetTabs", () => {
  test("opened ajoute en fin et active", () => {
    const s = opened(opened(NO_TABS, "kpi"), "quote");
    expect(names(s)).toEqual(["kpi", "quote"]);
    expect(activeName(s)).toBe("quote");
  });

  test("opened d'un nom déjà ouvert l'active sans doublon", () => {
    const s = opened(opened(opened(NO_TABS, "kpi"), "quote"), "kpi");
    expect(names(s)).toEqual(["kpi", "quote"]);
    expect(activeName(s)).toBe("kpi");
  });

  test("closed de l'actif au milieu active son voisin de gauche", () => {
    let s = opened(opened(opened(NO_TABS, "a"), "b"), "c");
    s = closed(activated(s, s.tabs[1].id), s.tabs[1].id);
    expect(names(s)).toEqual(["a", "c"]);
    expect(activeName(s)).toBe("a");
  });

  // Décision : à gauche du premier sous-onglet, il y a « Modèles ».
  test("closed du premier actif revient à Modèles", () => {
    let s = opened(opened(NO_TABS, "a"), "b");
    s = closed(activated(s, s.tabs[0].id), s.tabs[0].id);
    expect(s.active).toBeNull();
  });

  test("closed d'un autre que l'actif garde l'actif", () => {
    let s = opened(opened(NO_TABS, "a"), "b");
    s = closed(s, s.tabs[0].id);
    expect(activeName(s)).toBe("b");
  });

  test("renamed garde l'id et la place", () => {
    const s = opened(opened(NO_TABS, "a"), "b");
    const r = renamed(s, "a", "z");
    expect(names(r)).toEqual(["z", "b"]);
    expect(r.tabs[0].id).toBe(s.tabs[0].id);
  });

  test("pruned retire les noms disparus ; actif disparu → Modèles", () => {
    const s = pruned(opened(opened(NO_TABS, "a"), "b"), new Set(["a"]));
    expect(names(s)).toEqual(["a"]);
    expect(s.active).toBeNull();
  });

  test("pruned sans changement rend le même objet", () => {
    const s = opened(NO_TABS, "a");
    expect(pruned(s, new Set(["a"]))).toBe(s);
  });

  test("withRestored sur rien puis kept : aller-retour", () => {
    expect(kept(withRestored(NO_TABS, ["_commun", "kpi"], "kpi"))).toEqual({ widgets: ["_commun", "kpi"], widget: "kpi" });
  });

  test("withRestored garde un onglet déjà ouvert et son activité", () => {
    const s = withRestored(opened(NO_TABS, "quote"), ["kpi"], "kpi");
    expect(names(s)).toEqual(["quote", "kpi"]);
    expect(activeName(s)).toBe("quote");
  });

  test("withRestored dédoublonne et ignore un actif absent", () => {
    const s = withRestored(NO_TABS, ["kpi", "kpi"], "autre");
    expect(names(s)).toEqual(["kpi"]);
    expect(s.active).toBeNull();
  });

  test("après withRestored, opened donne un id neuf", () => {
    const s = opened(withRestored(NO_TABS, ["a", "b"], null), "c");
    expect(new Set(s.tabs.map((t) => t.id)).size).toBe(3);
  });
});
