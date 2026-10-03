import { afterAll, beforeEach, describe, expect, test } from "bun:test";
import { applyMessages, availableLocales, chosenLocale, direction, languageName, locale, onLocaleChange, setMessages, t } from "./i18n";
import { resetMessages } from "./test/messages";

const en = "ui-hello = Hello { $name }\nui-only-en = Only in English\n";
const fr = "ui-hello = Bonjour { $name }\n";

// Les autres fichiers de la passe attendent les identifiants bruts.
afterAll(resetMessages);

describe("t", () => {
  beforeEach(() => setMessages("fr", [en, fr]));

  test("dit le message dans la langue choisie, avec ses paramètres, sans marques d'isolement", () => {
    expect(t("ui-hello", { name: "Ada" })).toBe("Bonjour Ada");
    expect(locale()).toBe("fr");
  });

  test("un message absent de la langue vient de l'anglais", () => {
    expect(t("ui-only-en")).toBe("Only in English");
  });

  test("un identifiant inconnu se voit tel quel", () => {
    expect(t("ui-missing")).toBe("ui-missing");
  });
});

test("avant le chargement, t rend l'identifiant", () => {
  setMessages("en", []);
  expect(t("ui-hello")).toBe("ui-hello");
});

test("direction : de droite à gauche pour l'arabe, de gauche à droite sinon", () => {
  expect(direction("ar")).toBe("rtl");
  expect(direction("ar-EG")).toBe("rtl");
  expect(direction("fr")).toBe("ltr");
  expect(direction("zh-Hant")).toBe("ltr");
});

describe("choix de la langue", () => {
  test("languageName nomme chaque langue dans elle-même, avec une majuscule", () => {
    expect(languageName("fr")).toBe("Français");
    expect(languageName("de")).toBe("Deutsch");
  });

  test("applyMessages retient les langues livrées et le choix, puis prévient", () => {
    let told = 0;
    const stop = onLocaleChange(() => told++);
    applyMessages({ locale: "de", resources: [en], available: ["en", "de"], chosen: "de" });
    stop();
    expect(availableLocales()).toEqual(["en", "de"]);
    expect(chosenLocale()).toBe("de");
    expect(locale()).toBe("de");
    expect(told).toBe(1);
  });
});
