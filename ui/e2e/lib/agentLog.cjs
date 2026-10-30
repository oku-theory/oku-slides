// The fake agent's log (E2E_AGENT_LOG): what it received, one JSON line per message.
const assert = require('node:assert/strict');
const { existsSync, readFileSync, statSync } = require('node:fs');
const { SLOW } = require('./until.cjs');

/** The size of log `log` (0 before the agent's first turn writes it): what follows comes from the next action. */
const logOffset = (log) => (existsSync(log) ? statSync(log).size : 0);

/**
 * The messages noted in `log` since `from` (bytes), in order. The last line, maybe still being
 * written (no final newline), is not read; an unreadable line is skipped.
 */
const logSince = (log, from) => {
  if (!existsSync(log)) return [];
  const lines = readFileSync(log).subarray(from).toString('utf8').split('\n');
  lines.pop();
  return lines.flatMap((l) => {
    try {
      return [JSON.parse(l)];
    } catch {
      return [];
    }
  });
};

/** The MCP results (`mcp/result`) noted in `log` for `tool` since `from`, in order. */
const mcpResults = (log, from, tool) =>
  logSince(log, from)
    .filter((m) => m.method === 'mcp/result' && m.params.tool === tool)
    .map((m) => m.params.out);

/**
 * Waits for the first MCP result of `tool` noted in E2E_AGENT_LOG since `from` (bytes) and returns it,
 * decoded (CallToolResult). Throws at once with the result's text when it is an error (`isError`)
 * or unreadable, and after `timeout` ms when none comes.
 */
const waitForTool = async (p, from, tool, timeout = SLOW) => {
  const log = process.env.E2E_AGENT_LOG;
  assert.ok(log, 'E2E_AGENT_LOG manquant : lancer par just e2e');
  const t0 = Date.now();
  while (Date.now() - t0 < timeout) {
    const [out] = mcpResults(log, from, tool);
    if (out !== undefined) {
      let result;
      try {
        result = JSON.parse(out);
      } catch {
        assert.fail(`${tool} : résultat illisible : ${out.slice(0, 300)}`);
      }
      const text = (result.content ?? []).map((c) => c.text ?? '').join('\n');
      if (result.isError === true || result.is_error === true) assert.fail(`${tool} refusé : ${text}`);
      return result;
    }
    await p.waitForTimeout(100);
  }
  assert.fail(`aucun résultat MCP pour ${tool}`);
};

module.exports = { logOffset, logSince, mcpResults, waitForTool };
