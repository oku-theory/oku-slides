import { afterAll, afterEach, expect, test } from "bun:test";
import { type AgentUi, type AgentUiSent, agentApi } from "./api";
import { setMessages } from "./i18n";
import { resetMessages } from "./test/messages";

const UI: AgentUi = { slide: "s-1", element: null, field: null, editing: false, widget: null };
const realFetch = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = realFetch;
});
afterAll(resetMessages);

/** Le corps de la requête que `agentApi(...).prompt` envoie à l'hôte. */
async function sentPrompt(ui: AgentUi = UI): Promise<{ ui: AgentUiSent }> {
  let body = "";
  globalThis.fetch = (async (_url: string, init?: RequestInit) => {
    body = String(init?.body);
    return new Response("{}", { status: 200 });
  }) as unknown as typeof fetch;
  await agentApi("t-1").prompt("plus court", ui, null);
  return JSON.parse(body);
}

test("prompt envoie la langue que l'interface affiche au moment de la consigne", async () => {
  setMessages("de", []);
  expect((await sentPrompt()).ui.locale).toBe("de");
  setMessages("ja", []);
  expect((await sentPrompt()).ui.locale).toBe("ja");
});

test("prompt garde tout ce que voit l'utilisateur et y ajoute la langue", async () => {
  setMessages("fr", []);
  expect((await sentPrompt()).ui).toEqual({ ...UI, locale: "fr" });
});

test("prompt remplace une langue périmée portée par l'appelant", async () => {
  setMessages("de", []);
  const stale = { ...UI, locale: "fr" } as AgentUi;
  expect((await sentPrompt(stale)).ui.locale).toBe("de");
});

test("agentApi context names its window to the dev server, with the page and the seq", async () => {
  setMessages("fr", []);
  let sent: { body: unknown; window: string | null } = { body: null, window: null };
  globalThis.fetch = (async (_url: string, init?: RequestInit) => {
    sent = { body: JSON.parse(String(init?.body)), window: new Headers(init?.headers).get("X-Oku-Window") };
    return new Response("{}", { status: 200 });
  }) as unknown as typeof fetch;
  await agentApi("t-1").context("p-1", UI, 3, true, "w-1");
  expect(sent).toEqual({
    body: { cmd: "context", tab: "t-1", page: "p-1", ui: { ...UI, locale: "fr" }, seq: 3, has_focus: true },
    window: "w-1",
  });
});
