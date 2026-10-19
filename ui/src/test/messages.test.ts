// Garde (t-436) : la langue de l'interface est un état de module, partagé par
// tous les fichiers d'une même passe de `bun test`. Un fichier qui installe
// des messages doit les retirer après lui, sinon les fichiers suivants voient
// « Common » là où ils attendent l'identifiant `ui-widgets-common`.

import { afterAll, expect, test } from "bun:test";
import { applyMessages, locale, receiveMessages, t } from "../i18n";
import { resetMessages } from "./messages";

afterAll(resetMessages);

test("resetMessages remet la langue d'avant chargement", () => {
  applyMessages({ locale: "de", resources: ["ui-hello = Hallo\n"], chosen: "de" });
  resetMessages();
  expect([t("ui-hello"), locale()]).toEqual(["ui-hello", "en"]);
  // Choix remis à « langue du système » : des messages anglais sans choix ne changent rien.
  expect(receiveMessages({ locale: "en", resources: [], chosen: null })).toBe(false);
});

test("chaque fichier de test qui installe des messages enregistre resetMessages après lui", async () => {
  const src = `${import.meta.dir}/..`;
  const leaking: string[] = [];
  for await (const file of new Bun.Glob("**/*.test.{ts,tsx}").scan(src)) {
    const code = await Bun.file(`${src}/${file}`).text();
    if (!/\b(setMessages|applyMessages)\(/.test(code)) continue;
    if (!/\bafter(All|Each)\(resetMessages\)/.test(code)) leaking.push(file);
  }
  expect(leaking).toEqual([]);
});
