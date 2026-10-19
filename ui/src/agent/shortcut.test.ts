// Reconnaissance du raccourci « thème » par l'interface : mêmes cas que shortcut_tests.rs.

import { afterAll, describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { applyMessages } from "../i18n";
import { resetMessages } from "../test/messages";
import { isShortcut, normalizeShortcut } from "./shortcut";

/** Charge `ui.ftl` de `locale` (anglais d'abord, comme le cœur). */
const load = (locale: string) => {
  const ftl = (l: string) => readFileSync(`${import.meta.dir}/../../../crates/oku-slides-i18n/src/locales/${l}/ui.ftl`, "utf8");
  applyMessages({ locale, resources: locale === "en" ? [ftl("en")] : [ftl("en"), ftl(locale)], chosen: null });
};
afterAll(resetMessages);

describe("normalizeShortcut", () => {
  test.each([
    ["Thème.", "theme"],
    ["  THÈMES ! ", "themes"],
    ["«thème»", "theme"],
    ["thema’s", "thema's"],
    ["chủ   đề", "chu đe"],
    ["थीम।", "थीम"],
  ])("normalizeShortcut(%p) gives %p", (text, expected) => {
    expect(normalizeShortcut(text)).toBe(expected);
  });
});

describe("isShortcut", () => {
  test.each([
    ["fr", "Thème.", true],
    ["fr", "thèmes", true],
    ["fr", "THEME", true],
    ["fr", "change le thème en sombre", false],
    ["fr", "", false],
    ["fr", "   ", false],
    ["fr", "…", false],
    ["de", "Themen", true],
    ["ja", "テーマ。", true],
    ["hi", "थीम", true],
    ["hi", "थम", false],
    ["th", "ธีม", true],
    ["nl", "thema’s", true],
    ["ru", "Theme", true],
  ])("in %p, isShortcut(%p) is %p", (locale, text, expected) => {
    load(locale as string);
    expect(isShortcut(text as string)).toBe(expected as boolean);
  });

  test("a prompt over 64 bytes is never a shortcut", () => {
    load("en");
    expect(isShortcut(`theme${" ".repeat(100)}`)).toBe(false);
  });
});
