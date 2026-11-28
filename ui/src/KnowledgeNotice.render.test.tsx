// The deck tab's banner for received knowledge, rendered in a happy-dom DOM.

import { afterAll, afterEach, expect, test } from "bun:test";
import "./test/dom";
import { resetMessages } from "./test/messages";
import type { KnowledgeSummary } from "./api";
import type { KnowledgeLimitSlug } from "./knowledge/types";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: KnowledgeNotice, overLimitText } = await import("./KnowledgeNotice");
const { setMessages } = await import("./i18n");
const { readdirSync, readFileSync } = await import("node:fs");

const LOCALES = new URL("../../crates/oku-slides-i18n/src/locales/", import.meta.url);
const ftl = (lang: string) => readFileSync(new URL(`${lang}/ui.ftl`, LOCALES), "utf8");
const english = () => setMessages("en", [ftl("en")]);

afterAll(resetMessages);

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  english();
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function render(summary: KnowledgeSummary | null, reviews: string[] = []) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const onReview = () => void reviews.push("review");
  await act(async () => root.render(createElement(KnowledgeNotice, { summary, onReview })));
  return host;
}
const texts = (host: HTMLElement, selector: string) =>
  [...host.querySelectorAll(selector)].map((e) => e.textContent);
const ALL: KnowledgeLimitSlug[] = ["too-many-files", "file-too-large", "total-too-large"];

test("KnowledgeNotice one awaiting file says one", async () => {
  english();
  const host = await render({ awaiting: 1, over_limit: [] });
  expect(texts(host, ".knowledge-notice-text")).toEqual([
    "This deck holds one knowledge file you have not approved: the agent cannot read them.",
  ]);
});

test("KnowledgeNotice several awaiting files gives their count", async () => {
  english();
  const host = await render({ awaiting: 3, over_limit: [] });
  expect(texts(host, ".knowledge-notice-text")).toEqual([
    "This deck holds 3 knowledge files you have not approved: the agent cannot read them.",
  ]);
});

test("KnowledgeNotice over limit alone shows no awaiting sentence", async () => {
  english();
  const host = await render({ awaiting: 0, over_limit: ["too-many-files"] });
  expect(texts(host, ".knowledge-notice-text")).toEqual([
    "The knowledge base is past its limits (\u2068too many files\u2069). Only deleting files"
      + " brings it back under: ask the agent to delete some.",
  ]);
});

test("KnowledgeNotice Review opens the dialog", async () => {
  english();
  const reviews: string[] = [];
  const host = await render({ awaiting: 0, over_limit: ["total-too-large"] }, reviews);
  const review = host.querySelector("button") as HTMLButtonElement;
  expect(review.textContent).toBe("Review…");
  await act(async () => void review.dispatchEvent(new MouseEvent("click", { bubbles: true })));
  expect(reviews).toEqual(["review"]);
});

test("KnowledgeNotice null summary renders nothing", async () => {
  english();
  const host = await render(null);
  expect(host.innerHTML).toBe("");
});

test("overLimitText in French joins the limits as a French list", () => {
  setMessages("fr", [ftl("en"), ftl("fr")]);
  expect(overLimitText(ALL)).toBe(
    "La base de connaissances dépasse ses limites (\u2068trop de fichiers, un fichier trop gros"
      + " et taille totale trop grande\u2069). Seule la suppression de fichiers la ramène en"
      + " dessous : demandez à l'agent d'en supprimer.",
  );
});

test("overLimitText in Japanese joins the limits as a Japanese list", () => {
  setMessages("ja", [ftl("en"), ftl("ja")]);
  const list = "ファイルが多すぎる、大きすぎるファイルがある、合計サイズが大きすぎる";
  expect(overLimitText(ALL)).toContain(`\u2068${list}\u2069`);
});

// The slugs Rust's `KnowledgeLimit::slug` sends (pinned there too): each names its own message.
test("overLimitText each Rust slug names its ui-knowledge-limit message", () => {
  english();
  const english_names = ftl("en");
  for (const slug of ["too-many-files", "file-too-large", "total-too-large"] as const) {
    const line = english_names.split("\n").find((l) => l.startsWith(`ui-knowledge-limit-${slug} = `));
    expect(line).toBeDefined();
    const name = (line as string).split(" = ")[1];
    expect(overLimitText([slug])).toContain(`\u2068${name}\u2069`);
  }
});

test("overLimitText an unknown limit throws", () => {
  english();
  expect(() => overLimitText(["bogus" as KnowledgeLimitSlug])).toThrow();
});

// `locale()` is the catalog's folder name: each one must be a tag `Intl.ListFormat` takes.
test("overLimitText every catalog locale is a valid list format tag", () => {
  for (const lang of readdirSync(LOCALES)) {
    setMessages(lang, [ftl("en"), ftl(lang)]);
    expect(Intl.getCanonicalLocales(lang)).toEqual([lang]);
    expect(overLimitText(ALL)).toContain("⁨");
  }
});
