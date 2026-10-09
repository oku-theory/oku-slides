// Un transport `agent-link` en mémoire : ce qui est envoyé est livré à tous les écouteurs.

import type { LinkMessage, LinkTransport } from "../agent/link";

export function memoryTransport(): LinkTransport & { sent: LinkMessage[]; listeners: () => number } {
  const listeners = new Set<(m: LinkMessage) => void>();
  const sent: LinkMessage[] = [];
  return {
    sent,
    listeners: () => listeners.size,
    ready: Promise.resolve(),
    close: () => listeners.clear(),
    send: (m) => {
      sent.push(m);
      [...listeners].forEach((cb) => cb(m));
    },
    listen: (cb) => {
      listeners.add(cb);
      return () => void listeners.delete(cb);
    },
  };
}
