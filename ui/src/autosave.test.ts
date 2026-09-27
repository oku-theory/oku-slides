import { describe, expect, test } from "bun:test";
import { createAutosave, wantsAutosave } from "./autosave";

const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

describe("enregistrement automatique", () => {
  const deck = { dirty: true, path: "/d.oku-slides.pdf", editing: null };

  test("wantsAutosave deck modifié avec fichier : oui", () => {
    expect(wantsAutosave(deck, false)).toBe(true);
  });

  test("wantsAutosave deck sans modification : non", () => {
    expect(wantsAutosave({ ...deck, dirty: false }, false)).toBe(false);
  });

  test("wantsAutosave deck sans fichier : oui (il ira au dossier par défaut)", () => {
    expect(wantsAutosave({ ...deck, path: null }, false)).toBe(true);
  });

  test("wantsAutosave en pause (fichier remplacé, refusé) : non", () => {
    expect(wantsAutosave(deck, true)).toBe(false);
  });

  test("wantsAutosave modèles modifiés : oui ; modèles intégrés : non (il faut une copie)", () => {
    expect(wantsAutosave({ dirty: true, path: null, editing: { builtin: false } }, false)).toBe(true);
    expect(wantsAutosave({ dirty: true, path: null, editing: { builtin: true } }, false)).toBe(false);
  });

  test("createAutosave plusieurs changements rapprochés : un seul enregistrement", async () => {
    let saves = 0;
    const a = createAutosave(async () => void saves++, 20);
    a.schedule();
    await wait(5);
    a.schedule();
    await wait(5);
    a.schedule();
    await wait(40);
    expect(saves).toBe(1);
  });

  test("createAutosave rien avant le délai", async () => {
    let saves = 0;
    const a = createAutosave(async () => void saves++, 30);
    a.schedule();
    await wait(10);
    expect(saves).toBe(0);
    a.cancel();
  });

  test("createAutosave flush enregistre tout de suite ce qui attend", async () => {
    let saves = 0;
    const a = createAutosave(async () => void saves++, 1000);
    a.schedule();
    await a.flush();
    expect(saves).toBe(1);
  });

  test("createAutosave flush sans rien en attente n'enregistre pas", async () => {
    let saves = 0;
    const a = createAutosave(async () => void saves++, 1000);
    await a.flush();
    expect(saves).toBe(0);
  });

  test("createAutosave jamais deux enregistrements à la fois ; le changement pendant l'écriture suit", async () => {
    let running = 0;
    let most = 0;
    let saves = 0;
    const a = createAutosave(async () => {
      running++;
      most = Math.max(most, running);
      await wait(30);
      saves++;
      running--;
    }, 5);
    a.schedule();
    await wait(15);
    a.schedule();
    await wait(80);
    expect(most).toBe(1);
    expect(saves).toBe(2);
  });

  test("createAutosave flush pendant une écriture attend aussi celle qui suit", async () => {
    let saves = 0;
    const a = createAutosave(async () => {
      await wait(20);
      saves++;
    }, 1);
    a.schedule();
    await wait(5);
    a.schedule();
    await wait(3);
    await a.flush();
    expect(saves).toBe(2);
  });

  test("createAutosave une erreur n'arrête pas les suivants", async () => {
    let calls = 0;
    const a = createAutosave(async () => {
      calls++;
      if (calls === 1) throw new Error("disque");
    }, 5);
    a.schedule();
    await wait(20);
    a.schedule();
    await wait(20);
    expect(calls).toBe(2);
  });
});
