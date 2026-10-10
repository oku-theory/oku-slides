import { afterEach, describe, expect, test } from "bun:test";
import "../test/dom";
import { manualTimers } from "../test/timers";

const { StrictMode, act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useDraft, DRAFT_TYPING_MS } = await import("./useDraft");

let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  await cleanup?.();
  cleanup = null;
});

async function mount({ strict = false, slow = false, fail = false }: { strict?: boolean; slow?: boolean; fail?: boolean } = {}) {
  const timers = manualTimers();
  const sent: (string | null)[] = [];
  // Origines dans l'ordre des envois (une par envoi).
  const origins: string[] = [];
  const refusing = { on: fail };
  let release: () => void = () => {};
  const setDraft = (text: string | null, origin: string) => {
    origins.push(origin);
    if (refusing.on) return Promise.reject(new Error("no agent"));
    if (!slow) return Promise.resolve(void sent.push(text));
    return new Promise<void>((resolve) => {
      release = () => {
        sent.push(text);
        resolve();
      };
    });
  };
  let draft: ReturnType<typeof useDraft> | null = null;
  const Probe = () => {
    draft = useDraft(setDraft, timers);
    return null;
  };
  const root = createRoot(document.createElement("div"));
  const tree = strict ? createElement(StrictMode, null, createElement(Probe)) : createElement(Probe);
  await act(async () => root.render(tree));
  cleanup = async () => act(async () => root.unmount());
  return { timers, sent, origins, refusing, draft: () => draft!, release: () => release(), unmount: () => act(async () => root.unmount()) };
}

describe("useDraft", () => {
  test("useDraft append sous StrictMode envoie une seule fois", async () => {
    const m = await mount({ strict: true });
    await act(async () => m.draft().append("bonjour"));
    expect(m.sent).toEqual(["bonjour"]);
  });

  test("useDraft append ajoute le texte et compte la dictée", async () => {
    const m = await mount();
    await act(async () => m.draft().append("bonjour"));
    expect(m.draft().state).toEqual({ text: "bonjour", dictations: 1, typing: 0 });
  });

  test("useDraft type ouvre un brouillon vide et l'envoie tout de suite", async () => {
    const m = await mount();
    await act(async () => m.draft().type());
    expect(m.draft().state.text).toBe("");
    expect(m.sent).toEqual([""]);
  });

  test("useDraft change attend le délai de frappe avant d'envoyer", async () => {
    const m = await mount();
    await act(async () => m.draft().change("a"));
    expect(m.sent).toEqual([]);
    expect(m.timers.delays.at(-1)).toBe(DRAFT_TYPING_MS);
  });

  test("useDraft change en rafale n'envoie que le dernier texte", async () => {
    const m = await mount();
    await act(async () => m.draft().change("a"));
    await act(async () => m.draft().change("ab"));
    await act(async () => m.timers.run());
    expect(m.sent).toEqual(["ab"]);
  });

  test("useDraft close envoie null et oublie la frappe en attente", async () => {
    const m = await mount();
    await act(async () => m.draft().change("a"));
    await act(async () => m.draft().close());
    await act(async () => m.timers.run());
    expect(m.sent).toEqual([null]);
  });

  test("useDraft brouillon de l'hôte venu d'une autre vue remplace le texte", async () => {
    const m = await mount();
    await act(async () => m.draft().onHost({ text: "de l'autre vue", origin: "ailleurs" }));
    expect(m.draft().state.text).toBe("de l'autre vue");
  });

  test("useDraft brouillon rejoué par l'hôte (sans origine) remplace le texte", async () => {
    const m = await mount();
    await act(async () => m.draft().onHost({ text: "rejoué", origin: null }));
    expect(m.draft().state.text).toBe("rejoué");
  });

  test("useDraft l'écho d'un envoi de cette vue est ignoré", async () => {
    const m = await mount();
    await act(async () => m.draft().append("a"));
    await act(async () => m.draft().close());
    const own = m.origins[0]!;
    await act(async () => m.draft().onHost({ text: "a", origin: own }));
    expect(m.draft().state.text).toBeNull();
  });

  test("useDraft brouillon de l'hôte pendant une frappe en attente est ignoré", async () => {
    const m = await mount();
    await act(async () => m.draft().change("ici"));
    await act(async () => m.draft().onHost({ text: "ailleurs", origin: "autre" }));
    expect(m.draft().state.text).toBe("ici");
  });

  test("useDraft brouillon de l'hôte après l'envoi de la frappe est appliqué", async () => {
    const m = await mount();
    await act(async () => m.draft().change("ici"));
    await act(async () => m.timers.run());
    const own = m.origins[0]!;
    await act(async () => m.draft().onHost({ text: "ici", origin: own }));
    await act(async () => m.draft().onHost({ text: "ailleurs", origin: "autre" }));
    expect(m.draft().state.text).toBe("ailleurs");
  });

  test("useDraft brouillon d'ailleurs reçu avant l'écho de la frappe est ignoré", async () => {
    const m = await mount();
    await act(async () => m.draft().change("ici"));
    await act(async () => m.timers.run());
    await act(async () => m.draft().onHost({ text: "ailleurs", origin: "autre" }));
    expect(m.draft().state.text).toBe("ici");
  });

  test("useDraft au démontage la frappe en attente part", async () => {
    const m = await mount();
    await act(async () => m.draft().change("dernier mot"));
    await m.unmount();
    cleanup = null;
    expect(m.sent).toEqual(["dernier mot"]);
  });

  test("useDraft les envois partent dans l'ordre, un à la fois", async () => {
    const m = await mount({ slow: true });
    await act(async () => m.draft().append("un"));
    await act(async () => m.draft().close());
    expect(m.sent).toEqual([]);
    await act(async () => m.release());
    expect(m.sent).toEqual(["un"]);
    await act(async () => m.release());
    expect(m.sent).toEqual(["un", null]);
  });

  test("useDraft brouillon d'ailleurs reçu avant l'écho de cette vue est ignoré", async () => {
    const m = await mount();
    await act(async () => m.draft().append("mien"));
    await act(async () => m.draft().onHost({ text: "plus ancien", origin: "autre" }));
    expect(m.draft().state.text).toBe("mien");
  });

  test("useDraft brouillon d'ailleurs reçu après l'écho de cette vue est appliqué", async () => {
    const m = await mount();
    await act(async () => m.draft().append("mien"));
    const own = m.origins[0]!;
    await act(async () => m.draft().onHost({ text: "mien", origin: own }));
    await act(async () => m.draft().onHost({ text: "plus récent", origin: "autre" }));
    expect(m.draft().state.text).toBe("plus récent");
  });

  test("useDraft un envoi en échec ne bloque pas les brouillons suivants", async () => {
    const failing = await mount({ fail: true });
    await act(async () => failing.draft().append("refusé"));
    await act(async () => failing.draft().onHost({ text: "d'ailleurs", origin: "autre" }));
    expect(failing.draft().state.text).toBe("d'ailleurs");
  });

  test("useDraft écho perdu puis rejeu de l'hôte : le texte de cette vue est renvoyé", async () => {
    const m = await mount();
    await act(async () => m.draft().append("mien"));
    await act(async () => m.draft().onHost({ text: "ancien", origin: null }));
    expect(m.sent).toEqual(["mien", "mien"]);
  });

  test("useDraft écho perdu puis rejeu : le brouillon suivant d'ailleurs est appliqué", async () => {
    const m = await mount();
    await act(async () => m.draft().append("mien"));
    await act(async () => m.draft().onHost({ text: "ancien", origin: null }));
    await act(async () => m.draft().onHost({ text: "mien", origin: m.origins.at(-1)! }));
    await act(async () => m.draft().onHost({ text: "plus récent", origin: "autre" }));
    expect(m.draft().state.text).toBe("plus récent");
  });

  test("useDraft flush se résout une fois la frappe envoyée", async () => {
    const m = await mount({ slow: true });
    await act(async () => m.draft().change("dernier mot"));
    let done = false;
    const flushed = m.draft().flush().then(() => void (done = true));
    // L'envoi part au tour suivant de la file : laisser `setDraft` poser `release`.
    await act(async () => {});
    expect(done).toBe(false);
    await act(async () => m.release());
    await act(async () => flushed);
    expect(done).toBe(true);
    expect(m.sent).toEqual(["dernier mot"]);
  });
  test("useDraft chaque envoi porte sa propre origine, de même préfixe", async () => {
    const m = await mount();
    await act(async () => m.draft().append("un"));
    await act(async () => m.draft().close());
    const [a, b] = m.origins;
    expect(a).not.toBe(b);
    expect(a!.split(":")[0]).toBe(b!.split(":")[0]!);
  });

  test("useDraft écho en retard après un rejeu : ce qui vient d'ailleurs attend l'écho du renvoi", async () => {
    const m = await mount();
    await act(async () => m.draft().append("mien"));
    await act(async () => m.draft().onHost({ text: "ancien", origin: null }));
    // L'écho du premier envoi n'était qu'en retard : celui du renvoi manque encore.
    await act(async () => m.draft().onHost({ text: "mien", origin: m.origins[0]! }));
    await act(async () => m.draft().onHost({ text: "plus ancien", origin: "autre" }));
    expect(m.draft().state.text).toBe("mien");
  });

  test("useDraft dernier envoi refusé puis rejeu de l'hôte : le brouillon local est gardé", async () => {
    const m = await mount({ fail: true });
    await act(async () => m.draft().type());
    m.refusing.on = false;
    await act(async () => m.draft().onHost({ text: null, origin: null }));
    expect(m.draft().state.text).toBe("");
  });

  test("useDraft dernier envoi refusé puis rejeu de l'hôte : le brouillon local est renvoyé", async () => {
    const m = await mount({ fail: true });
    await act(async () => m.draft().append("dicté tôt"));
    m.refusing.on = false;
    await act(async () => m.draft().onHost({ text: null, origin: null }));
    expect(m.sent).toEqual(["dicté tôt"]);
  });

  test("useDraft envoi refusé puis brouillon d'ailleurs : le rejeu suivant est appliqué", async () => {
    const m = await mount({ fail: true });
    await act(async () => m.draft().append("refusé"));
    m.refusing.on = false;
    await act(async () => m.draft().onHost({ text: "d'ailleurs", origin: "autre" }));
    await act(async () => m.draft().onHost({ text: "rejoué", origin: null }));
    expect(m.draft().state.text).toBe("rejoué");
  });
});
