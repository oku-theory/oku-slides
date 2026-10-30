import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { appendFileSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { logOffset, waitForTool } from "./agentLog.cjs";

const page = {
  waitForTimeout: (ms: number) => new Promise((r) => setTimeout(r, ms)),
};
const line = (method: string, params: object) =>
  `${JSON.stringify({ method, params })}\n`;
const prompt = line("session/prompt", { sessionId: "s" });
const end = line("turn/end", { session: "s", stop: "end_turn" });
const result = (out: object) =>
  line("mcp/result", {
    session: "s",
    tool: "open_pack",
    out: JSON.stringify(out),
  });

let dir: string;
let log: string;
let saved: string | undefined;

beforeEach(() => {
  dir = mkdtempSync(join(tmpdir(), "agent-log-"));
  log = join(dir, "agent.log");
  saved = process.env.E2E_AGENT_LOG;
  process.env.E2E_AGENT_LOG = log;
});

afterEach(() => {
  if (saved === undefined) delete process.env.E2E_AGENT_LOG;
  else process.env.E2E_AGENT_LOG = saved;
  rmSync(dir, { recursive: true, force: true });
});

describe("waitForTool", () => {
  test("waitForTool with the tool called returns its decoded result", async () => {
    writeFileSync(log, "");
    const from = logOffset(log);
    appendFileSync(log, prompt + result({ ok: true }) + end);
    expect(await waitForTool(page, from, "open_pack", 2000)).toEqual({
      ok: true,
    });
  });

  test("waitForTool with the turn ended without the tool fails at once", async () => {
    writeFileSync(log, "");
    const from = logOffset(log);
    appendFileSync(log, prompt + end);
    const t0 = Date.now();
    await expect(waitForTool(page, from, "open_pack", 5000)).rejects.toThrow(
      "open_pack",
    );
    expect(Date.now() - t0).toBeLessThan(2000);
  });

  test("waitForTool with a turn end from before the action keeps waiting", async () => {
    writeFileSync(log, prompt + end);
    const from = logOffset(log);
    setTimeout(
      () => appendFileSync(log, prompt + result({ ok: true }) + end),
      300,
    );
    expect(await waitForTool(page, from, "open_pack", 2000)).toEqual({
      ok: true,
    });
  });

  test("waitForTool with a turn end before any prompt of this action keeps waiting", async () => {
    writeFileSync(log, "");
    const from = logOffset(log);
    appendFileSync(log, end);
    await expect(waitForTool(page, from, "open_pack", 400)).rejects.toThrow(
      "aucun résultat",
    );
  });

  test("waitForTool with a refused result throws its text", async () => {
    writeFileSync(log, "");
    const from = logOffset(log);
    appendFileSync(
      log,
      prompt +
        line("mcp/result", {
          tool: "open_pack",
          out: JSON.stringify({ isError: true, content: [{ text: "non" }] }),
        }),
    );
    await expect(waitForTool(page, from, "open_pack", 2000)).rejects.toThrow(
      "non",
    );
  });
});
