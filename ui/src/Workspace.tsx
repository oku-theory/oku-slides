// Une fenêtre : l'`App` de l'onglet actif, la barre d'onglets en bas.

import { useCallback, useEffect, useRef, useState } from "react";
import App from "./App";
import TabBar from "./TabBar";
import { errorText, inTauri, isMac, listenTauri, tabApi, tabDrag, workspace, type WorkspaceView } from "./api";
import { mayClose } from "./close";
import { t } from "./i18n";
import { cycle, nth, tabShortcut } from "./tabs";

/** Label de cette fenêtre : celui de Tauri, ou la seule fenêtre en développement. */
async function ownLabel(view: WorkspaceView): Promise<string> {
  if (!inTauri) return view.windows[0]?.label ?? "";
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  return getCurrentWindow().label;
}

/** Une fenêtre : sa barre d'onglets et l'`App` de l'onglet actif. */
export default function Workspace() {
  const [view, setView] = useState<WorkspaceView | null>(null);
  const [label, setLabel] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Composant démonté (StrictMode, ou fenêtre fermée) : on ignore les réponses tardives.
  const alive = useRef(true);
  useEffect(() => {
    // StrictMode monte, démonte puis remonte en développement : on redevient
    // vivant à chaque montage.
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);

  // Chaque vue diffusée par l'hôte fait relire son snapshot à l'`App` : un avis
  // posé sur l'onglet actif sans le changer (fichier reçu qui n'a pas pu
  // s'ouvrir) s'affiche aussitôt.
  const [nonce, setNonce] = useState(0);
  const bump = useCallback(() => setNonce((n) => n + 1), []);

  /** Relit la vue, sans relecture du snapshot : ce que fait l'`App` après chaque commande. */
  const reread = useCallback(() => {
    workspace.view().then((v) => alive.current && setView(v)).catch((e) => alive.current && setError(errorText(e)));
  }, []);

  /** Relit la vue, puis le snapshot de l'onglet actif. */
  const refresh = useCallback(() => {
    reread();
    bump();
  }, [reread, bump]);

  const run = useCallback(
    async (p: Promise<WorkspaceView>) => {
      try {
        const v = await p;
        if (alive.current) {
          setView(v);
          setError(null);
        }
      } catch (e) {
        if (alive.current) setError(errorText(e));
        refresh();
      }
    },
    [refresh],
  );

  // L'écoute d'abord, la vue de départ ensuite : aucune diffusion perdue entre les deux.
  useEffect(() => {
    const first = () =>
      workspace
        .view()
        .then(async (v) => {
          if (!alive.current) return;
          setView(v);
          const l = await ownLabel(v);
          if (alive.current) setLabel(l);
        })
        .catch((e) => alive.current && setError(errorText(e)));
    if (!inTauri) {
      first();
      return;
    }
    return listenTauri(async () => {
      const { listen } = await import("@tauri-apps/api/event");
      const stop = await listen<WorkspaceView>("workspace", (e) => {
        if (!alive.current) return;
        setView(e.payload);
        bump();
      });
      first();
      return stop;
    });
  }, []);

  const win = view?.windows.find((w) => w.label === label);
  const viewRef = useRef(view);
  viewRef.current = view;
  const winRef = useRef(win);
  winRef.current = win;

  /** La vue relue juste avant de demander : son « modifié » est à jour. */
  const freshTabs = async () => {
    const v = await workspace.view();
    if (alive.current) setView(v);
    return v.windows.find((w) => w.label === label)?.tabs ?? [];
  };

  // Un double clic sur la croix d'un onglet modifié ne pose la question qu'une fois.
  const closing = useRef(new Set<string>());

  /** Ferme un onglet après la question ; `false` : l'utilisateur a renoncé. */
  const closeTab = useCallback(
    async (id: string): Promise<boolean> => {
      if (closing.current.has(id)) return false;
      closing.current.add(id);
      try {
        const tab = (await freshTabs()).find((x) => x.id === id);
        if (!tab) return true;
        const verdict = await mayClose(tab, tabApi(id));
        if (!verdict) return false;
        await run(workspace.close(id, verdict === "discard"));
        return true;
      } catch (e) {
        if (alive.current) setError(errorText(e));
        return false;
      } finally {
        closing.current.delete(id);
      }
    },
    [run, label],
  );

  // « Annuler » sur un onglet arrête tout.
  const closeOthers = async (id: string) => {
    for (const tab of winRef.current?.tabs ?? []) {
      if (tab.id !== id && !(await closeTab(tab.id))) return;
    }
  };

  // Fermer la fenêtre (l'hôte l'a empêché et prévient) : la dernière quitte
  // en gardant tout ; une autre pose la question pour ses onglets modifiés.
  // Deux demandes rapprochées (double clic) ne posent pas deux fois les questions.
  const closingWindow = useRef(false);
  useEffect(() => {
    if (!inTauri || label === null) return;
    return listenTauri(async () => {
      const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
      const { invoke } = await import("@tauri-apps/api/core");
      return getCurrentWebviewWindow().listen("close-requested", async () => {
        if (closingWindow.current) return;
        closingWindow.current = true;
        // L'onglet actif au moment de la demande : à retrouver si l'utilisateur renonce.
        const activeBefore = winRef.current?.active;
        try {
          if ((viewRef.current?.windows.length ?? 0) <= 1) return void (await invoke("quit"));
          const discard: string[] = [];
          // Une question par onglet modifié, volontairement : chaque document a sa décision.
          for (const tab of await freshTabs()) {
            if (!tab.dirty) continue;
            await workspace.activate(tab.id);
            const verdict = await mayClose(tab, tabApi(tab.id));
            if (!verdict) {
              if (activeBefore) await workspace.activate(activeBefore);
              return;
            }
            if (verdict === "discard") discard.push(tab.id);
          }
          await workspace.closeWindow(label, discard);
        } catch (e) {
          if (alive.current) setError(errorText(e));
        } finally {
          closingWindow.current = false;
        }
      });
    });
  }, [label]);

  // Raccourcis des onglets.
  useEffect(() => {
    if (label === null) return;
    const onKey = (e: KeyboardEvent) => {
      const action = tabShortcut(e, isMac);
      const w = winRef.current;
      if (!action || !w) return;
      if (action.kind === "new-window" && !inTauri) return;
      // Pendant une saisie (texte d'une slide, champ), changer d'onglet
      // perdrait ce qui n'est pas validé : seul Ctrl+Tab reste actif.
      const typing = (e.target as HTMLElement | null)?.closest?.("input, textarea, select, [contenteditable]");
      if (typing && action.kind !== "cycle") return;
      e.preventDefault();
      const ids = w.tabs.map((x) => x.id);
      if (action.kind === "new-tab") run(workspace.newTab(label));
      else if (action.kind === "close-tab") {
        if (inTauri || ids.length > 1) closeTab(w.active);
      } else if (action.kind === "new-window") run(workspace.newWindow());
      else {
        const id = action.kind === "cycle" ? cycle(ids, w.active, action.delta) : nth(ids, action.n);
        if (id && id !== w.active) run(workspace.activate(id));
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [label, run, closeTab]);

  if (!view || label === null || !win) {
    return <div className="table"><p className="empty" style={{ padding: 40 }}>{error ?? t("ui-loading")}</p></div>;
  }

  // Un fichier ouvert devient l'onglet actif (nouveau, ou déjà ouvert
  // ailleurs) : `key` remonte l'`App`. Déjà actif : rien à recharger.
  const onOpen = (path: string) => void run(workspace.openTab(label, path));
  const dropOutside = async (id: string) => {
    try {
      await tabDrag.drop(id);
    } catch (e) {
      if (alive.current) setError(errorText(e));
    }
  };

  return (
    <div className="workspace">
      {/* Barre de titre intégrée (macOS) : de quoi déplacer la fenêtre, et ses boutons. */}
      {inTauri && isMac && <div className="titlebar" data-tauri-drag-region />}
      {error && <p className="workspace-error" role="alert">{error}</p>}
      <div className="workspace-body">
        {/* En développement, `onChange` relit la vue sans relancer le snapshot : pas de boucle. */}
        <App key={win.active} tab={win.active} nonce={nonce} onOpen={onOpen} onChange={inTauri ? undefined : reread} />
      </div>
      <TabBar
        view={view}
        win={win}
        windows={inTauri}
        onActivate={(id) => run(workspace.activate(id))}
        onClose={closeTab}
        onCloseOthers={closeOthers}
        onNew={() => run(workspace.newTab(label))}
        onReorder={(id, index) => run(workspace.reorder(id, index))}
        onMove={(id, target) => run(workspace.moveTab(id, target))}
        onDragOver={tabDrag.over}
        onDragLeave={tabDrag.leave}
        onDropOutside={dropOutside}
      />
    </div>
  );
}
