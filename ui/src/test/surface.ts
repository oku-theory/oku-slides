// Montage d'une surface A2UI dans le DOM de test, partagé par les tests de
// rendu du catalogue. À charger par `await import`, après `./dom` (react-dom
// cherche le DOM à son chargement). La surface montée est démontée après
// chaque test du fichier qui l'importe.

import { afterEach } from "bun:test";
import type { A2uiClientAction, A2uiMessage } from "@a2ui/web_core/v0_9";
import type { ReactElement } from "react";
import { act, createElement } from "react";
import { createRoot } from "react-dom/client";
import { CATALOG_ID } from "../a2ui/catalog";
import { SurfaceView } from "../a2ui/SurfaceView";

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

/** Messages d'une surface `s` du catalogue : création, composants, modèle de données. */
export const messagesOf = (components: unknown[], data: unknown = {}) =>
  [
    { version: "v0.9", createSurface: { surfaceId: "s", catalogId: CATALOG_ID } },
    { version: "v0.9", updateComponents: { surfaceId: "s", components } },
    { version: "v0.9", updateDataModel: { surfaceId: "s", value: data } },
  ] as unknown as A2uiMessage[];

/**
 * Monte la surface `s` ; renvoie son hôte, les actions et les erreurs reçues.
 * `wrap` l'entoure (un fournisseur de contexte, par exemple) ; `respond` voit chaque action
 * pendant son envoi, comme le routeur de l'app.
 */
export async function mountSurface(
  messages: A2uiMessage[],
  wrap: (surface: ReactElement) => ReactElement = (s) => s,
  respond: (a: A2uiClientAction) => void = () => {},
) {
  const actions: A2uiClientAction[] = [];
  const errors: string[] = [];
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => {
    root.render(
      wrap(
        createElement(SurfaceView, {
          messages,
          surfaceId: "s",
          onAction: (a: A2uiClientAction) => {
            actions.push(a);
            respond(a);
          },
          onError: (m: string) => errors.push(m),
        }),
      ),
    );
  });
  return { host, actions, errors };
}
