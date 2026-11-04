import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { appendFileSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { say } from "./say.cjs";

let dir: string;
let file: string;
let log: string;
const saved = { utterances: process.env.E2E_UTTERANCES, log: process.env.E2E_AGENT_LOG };

const promptLine = (text: string) => `${JSON.stringify({ method: "session/prompt", params: { prompt: [{ type: "text", text: "contexte" }, { type: "text", text }] } })}\n`;
const read = (path: string) => readFileSync(path, "utf8");

/** A fake page: listening once `listening` is true; it records the file seen while it was not. */
function page(state: { listening: boolean; seenWhileDeaf: string[] }) {
  return {
    waitForTimeout: (ms: number) => new Promise((r) => setTimeout(r, ms)),
    locator: () => ({
      count: async () => {
        if (!state.listening) state.seenWhileDeaf.push(read(file));
        return state.listening ? 1 : 0;
      },
    }),
  };
}

beforeEach(() => {
  dir = mkdtempSync(join(tmpdir(), "say-"));
  file = join(dir, "utterances.txt");
  log = join(dir, "agent.log");
  writeFileSync(file, "");
  writeFileSync(log, "");
  process.env.E2E_UTTERANCES = file;
  process.env.E2E_AGENT_LOG = log;
});

afterEach(() => {
  for (const [key, value] of [["E2E_UTTERANCES", saved.utterances], ["E2E_AGENT_LOG", saved.log]] as const) {
    if (value === undefined) delete process.env[key];
    else process.env[key] = value;
  }
  rmSync(dir, { recursive: true, force: true });
});

describe("say", () => {
  test("say waits for the listening state before appending the line", async () => {
    const state = { listening: false, seenWhileDeaf: [] as string[] };
    setTimeout(() => (state.listening = true), 250);
    setTimeout(() => appendFileSync(log, promptLine("bonjour tout le monde")), 400);
    await say(page(state), "bonjour tout le monde", { timeout: 3000 });
    expect(state.seenWhileDeaf.length).toBeGreaterThan(0);
    expect(state.seenWhileDeaf.every((seen) => seen === "")).toBe(true);
    expect(read(file)).toBe("bonjour tout le monde\n");
  });

  test("say returns the messages once the line is prompted", async () => {
    appendFileSync(log, promptLine("avant"));
    setTimeout(() => appendFileSync(log, promptLine("ajoute une slide")), 200);
    const since = await say(page({ listening: true, seenWhileDeaf: [] }), "ajoute une slide", { timeout: 3000 });
    expect(since.map((m: { params: { prompt: { text: string }[] } }) => m.params.prompt[1].text)).toEqual(["ajoute une slide"]);
  });

  test("say with nothing heard fails with its reason", async () => {
    const deaf = say(page({ listening: true, seenWhileDeaf: [] }), "stop", { heard: () => false, why: "no stop", timeout: 300 });
    await expect(deaf).rejects.toThrow("no stop");
  });
});
