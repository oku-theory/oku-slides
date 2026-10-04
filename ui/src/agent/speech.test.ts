// Ce que la région annoncée du fil lit d'une surface, et seulement ce qui est nouveau.

import { afterEach, beforeEach, describe, expect, jest, test } from "bun:test";
import type { Surface } from "../api";
import { realSurface as real } from "../a2ui/fixtures/agentSurfaces";
import type { Spoken } from "./speech";
import { newSpeech, QUIET_MS, Speech, spokenLines } from "./speech";

/** `s` avec un modèle de données remplacé. */
const withData = (s: Surface, value: Record<string, unknown>): Surface => ({
  ...s,
  messages: [...s.messages, { version: "v0.9", updateDataModel: { surfaceId: s.id, value } }] as unknown as Surface["messages"],
});

describe("spokenLines", () => {
  test("un tour : la consigne et les textes liés au modèle (narration, issue), pas la timeline", () => {
    const lines = spokenLines(withData(real("turn-3"), { narration: "Je change le fond.", outcome: "", steps: [{ label: "apply", status: "completed" }] }));
    expect(lines).toEqual(["Change le fond de la slide quatre.", "Je change le fond."]);
  });

  test("un reçu : titre et faits ; ni boutons ni détail replié", () => {
    expect(spokenLines(real("report-8"))).toEqual(["Slide 4 : fond bleu nuit", "titre en blanc", "surtitre bleu clair"]);
  });

  test("une carte render : textes et badge, pas le libellé du bouton", () => {
    expect(spokenLines(real("render-9"))).toEqual(["Trois palettes possibles", "nouveau"]);
  });
});

describe("newSpeech", () => {
  test("rien de nouveau : rien à lire", () => {
    expect(newSpeech(["a", "b"], ["a", "b"])).toBe("");
    expect(newSpeech(["a", "b"], ["a"])).toBe("");
  });

  test("une ligne nouvelle est lue", () => {
    expect(newSpeech(["Consigne"], ["Consigne", "Fait."])).toBe("Fait.");
  });

  test("une ligne qui grandit (narration en flux) : seule la suite est lue", () => {
    expect(newSpeech(["Je change"], ["Je change le fond."])).toBe("le fond.");
    expect(newSpeech(["Je", "Je change"], ["Je change le fond."])).toBe("le fond.");
  });
});

describe("newSpeech, lignes coupées", () => {
  test("une ligne coupée (« … ») dont le début est déjà lu n'apporte rien", () => {
    expect(newSpeech(["Je change le fond de la"], ["Je change le fo…"])).toBe("");
  });
  test("une ligne non coupée, début strict d'une ligne déjà lue, est lue", () => {
    expect(newSpeech(["Sable clair"], ["Sable"])).toBe("Sable");
  });
  test("la forme entière d'une ligne lue coupée : seule la suite", () => {
    expect(newSpeech(["Je change le fo…"], ["Je change le fond."])).toBe("nd.");
  });
});

describe("Speech", () => {
  let said: Spoken[][];
  let speech: Speech;
  const turn = (narration: string) => withData(real("turn-3"), { narration, outcome: "" });
  const texts = () => said.map((batch) => batch.map((x) => x.text).join(" | "));
  beforeEach(() => {
    jest.useFakeTimers();
    said = [];
    speech = new Speech((x) => said.push(x));
    speech.update([], [], 0);
  });
  afterEach(() => {
    speech.dispose();
    jest.useRealTimers();
  });

  test("une narration en flux se lit d'un coup à la fin de la phrase, pas morceau par morceau", () => {
    speech.update([turn("Je")], [], 0);
    speech.update([turn("Je chan")], [], 0);
    speech.update([turn("Je change le fo")], [], 0);
    expect(said).toEqual([]);
    speech.update([turn("Je change le fond.")], [], 0);
    expect(texts()).toEqual(["Change le fond de la slide quatre. Je change le fond."]);
    jest.advanceTimersByTime(QUIET_MS);
    expect(said).toHaveLength(1);
  });

  test("une phrase inachevée se lit après un silence", () => {
    speech.update([turn("Je change")], [], 0);
    jest.advanceTimersByTime(QUIET_MS - 1);
    speech.update([turn("Je change le fond")], [], 0);
    jest.advanceTimersByTime(QUIET_MS - 1);
    expect(said).toEqual([]);
    jest.advanceTimersByTime(1);
    expect(texts()).toEqual(["Change le fond de la slide quatre. Je change le fond"]);
  });

  test("une narration lue puis coupée par l'hôte (« … ») n'est pas relue", () => {
    speech.update([turn("Je change le fond de la")], [], 0);
    jest.advanceTimersByTime(QUIET_MS);
    speech.update([turn("Je change le fo…")], [], 0);
    jest.advanceTimersByTime(QUIET_MS);
    expect(said).toHaveLength(1);
  });

  test("une resynchronisation (nouvelle génération) tient tout pour lu, même après un fil vide", () => {
    speech.update([real("turn-3"), real("report-8")], [], 1);
    jest.advanceTimersByTime(QUIET_MS);
    expect(said).toEqual([]);
    // Ce qui arrive ensuite se lit.
    speech.update([real("turn-3"), real("report-8"), real("render-9")], [], 1);
    jest.advanceTimersByTime(QUIET_MS);
    expect(texts()).toEqual(["Trois palettes possibles nouveau"]);
  });

  test("une question du dock qui se fige dans le fil ne relit que sa réponse", () => {
    const live = { ...real("ask-1"), id: "ask-7", place: "thread", live: true } as Surface;
    speech.update([], [live], 0);
    speech.update([real("ask-7")], [], 0);
    jest.advanceTimersByTime(QUIET_MS);
    expect(texts()).toEqual(["✓ Deux colonnes"]);
  });
});
