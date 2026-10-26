// Journal du faux agent (E2E_AGENT_LOG) : ce qu'il a reçu, ligne JSON par message.
const { existsSync, readFileSync, statSync } = require('node:fs');

/** The size of log `log` (0 before the agent's first turn writes it): what follows comes from the next action. */
const logOffset = (log) => (existsSync(log) ? statSync(log).size : 0);

/**
 * Les messages notés dans `log` depuis `from` (octets), dans l'ordre. La dernière ligne,
 * peut-être en cours d'écriture (sans saut de ligne final), n'est pas lue ; une ligne illisible est ignorée.
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

module.exports = { logOffset, logSince, mcpResults };
