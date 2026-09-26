import { beforeEach, describe, expect, test } from "bun:test";
import { direction, locale, setMessages, t } from "./i18n";

const en = "ui-hello = Hello { $name }\nui-only-en = Only in English\n";
const fr = "ui-hello = Bonjour { $name }\n";

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
