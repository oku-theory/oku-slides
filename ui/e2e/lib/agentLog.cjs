// Journal du faux agent (E2E_AGENT_LOG) : ce qu'il a reçu, ligne JSON par message.
const { readFileSync, statSync } = require('node:fs');

/** La taille du journal `log` : ce qui s'écrit ensuite vient de l'action qui suit. */
const logOffset = (log) => statSync(log).size;

/**
 * Les messages notés dans `log` depuis `from` (octets), dans l'ordre. La dernière ligne,
 * peut-être en cours d'écriture (sans saut de ligne final), n'est pas lue ; une ligne illisible est ignorée.
 */
const logSince = (log, from) => {
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

module.exports = { logOffset, logSince };
