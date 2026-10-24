// Une fenêtre : l'`App` de l'onglet actif, la barre d'onglets en bas.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import App from "./App";
import TabBar from "./TabBar";
import UpdateBar from "./UpdateBar";
import { errorText, inTauri, isMac, listenTauri, tabApi, tabDrag, tauriUpdates, workspace, type ImportTarget, type WorkspaceView } from "./api";
import { mayClose } from "./close";
import { t } from "./i18n";
import { newest } from "./latest";
import { mergedNotice, openGuarded } from "./merge";
import { cycle, nth } from "./tabs";
import { useImportJob } from "./useImportJob";
import { useImportPick } from "./useImportPick";
import { useSharedClip } from "./useSharedClip";
import { useTabShortcuts } from "./useTabShortcuts";
import { ERROR_MS } from "./useToast";
import { useTransient } from "./useTransient";
import { useUpdate } from "./useUpdate";

/** Label de cette fenêtre : celui de Tauri, ou la seule fenêtre en développement. */
async function ownLabel(view: WorkspaceView): Promise<string> {
  if (!inTauri) return view.windows[0]?.label ?? "";
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  return getCurrentWindow().label;
}

/** Une fenêtre : sa barre d'onglets et l'`App` de l'onglet actif. */
export default function Workspace() {
  const [view, setShown] = useState<WorkspaceView | null>(null);
  // Réponses et événements peuvent arriver dans le désordre : jamais une vue plus ancienne.
  const setView = useCallback((v: WorkspaceView) => setShown((cur) => newest(cur, v)), []);
  const [label, setLabel] = useState<string | null>(null);
  // Une erreur s'efface d'elle-même, comme l'avis d'erreur d'un onglet : rien d'autre ne la retire
  // tant qu'aucune commande de la fenêtre ne réussit.
  const [error, showError] = useTransient<string>();
  const setError = useCallback((m: string | null) => showError(m, ERROR_MS), [showError]);
  // Mise à jour de l'app (hôte Tauri seulement) : bandeau ici ; la recherche passe par l'onglet actif,
  // dont l'avis dit le résultat (un seul avis par fenêtre).
  const update = useUpdate(tauriUpdates, setError);
  const appUpdates = useMemo(() => (tauriUpdates ? { check: update.check, requested: update.requested } : undefined), [update.check, update.requested]);
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
  // L'avis d'une fusion faite à l'ouverture : seule la réponse de la fusion
  // le porte, l'`App` de son onglet le montre (voir `openGuarded`).
  const [merged, setMerged] = useState<{ tab: string; text: string } | null>(null);

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

  // L'élément copié : à la fenêtre, pas à l'`App` remontée à chaque onglet, et partagé par
  // l'hôte avec les autres fenêtres (t-1182).
  const shared = useSharedClip(label, setError);

  const win = view?.windows.find((w) => w.label === label);
  // L'avis de fusion, montré par l'`App` de l'onglet actif (celui de la
  // fusion, ou l'actif d'ici si elle a eu lieu dans une autre fenêtre) ; ses
  // effets passent avant celui-ci, qui l'oublie une fois montré.
  const mergeNotice = win ? mergedNotice(merged, win) : null;
  useEffect(() => {
    if (mergeNotice !== null) setMerged(null);
  }, [mergeNotice]);
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

  /** Ferme un onglet, enregistré d'abord s'il est modifié ; `false` : pas fermé. */
  const closeTab = useCallback(
    async (id: string): Promise<boolean> => {
      if (closing.current.has(id)) return false;
      closing.current.add(id);
      try {
        const tab = (await freshTabs()).find((x) => x.id === id);
        if (!tab) return true;
        const verdict = await mayClose(tab, tabApi(id));
        if (!verdict) return false;
        await run(workspace.close(id, false));
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
          // Chaque onglet modifié est enregistré ; montré d'abord, s'il faut répondre à un conflit.
          for (const tab of await freshTabs()) {
            if (!tab.dirty) continue;
            await workspace.activate(tab.id);
            const verdict = await mayClose(tab, tabApi(tab.id));
            if (!verdict) {
              if (activeBefore) await workspace.activate(activeBefore);
              return;
            }
          }
          await workspace.closeWindow(label, []);
        } catch (e) {
          if (alive.current) setError(errorText(e));
        } finally {
          closingWindow.current = false;
        }
      });
    });
  }, [label]);

  // L'import d'un .pptx : son dialogue (dans l'`App` active) peut se fermer pendant qu'il tourne ;
  // un refus arrivé ensuite s'affiche ici.
  const imports = useImportJob(setError);
  // Un .pptx reçu de l'OS (argument, seconde instance, macOS) : la vue le donne à sa fenêtre,
  // qui ouvre le dialogue puis le fait oublier à l'hôte.
  // Refusé pendant un import, le choix est repris quand celui-ci se termine.
  useImportPick(view, label, imports.job?.busy ?? false, imports.pick, (id) => {
    workspace
      .importPicked(id)
      .then((v) => alive.current && setView(v))
      .catch((e) => alive.current && setError(errorText(e)));
  });

  // Raccourcis des onglets, retenus pendant que le dialogue d'import est ouvert.
  useTabShortcuts(label !== null, {
    mac: isMac,
    windows: inTauri,
    held: imports.holdsKeys,
    onAction: (action) => {
      const w = winRef.current;
      if (!w || label === null) return;
      const ids = w.tabs.map((x) => x.id);
      if (action.kind === "new-tab") run(workspace.newTab(label));
      else if (action.kind === "help") run(workspace.openHelp(label));
      else if (action.kind === "close-tab") {
        if (inTauri || ids.length > 1) closeTab(w.active);
      } else if (action.kind === "new-window") run(workspace.newWindow());
      else {
        const id = action.kind === "cycle" ? cycle(ids, w.active, action.delta) : nth(ids, action.n);
        if (id && id !== w.active) run(workspace.activate(id));
      }
    },
  });

  if (!view || label === null || !win) {
    return <div className="table"><p className="empty" style={{ padding: 40 }}>{error ?? t("ui-loading")}</p></div>;
  }

  // Un fichier ouvert devient l'onglet actif (nouveau, ou déjà ouvert
  // ailleurs) : `key` remonte l'`App`. Déjà actif : rien à recharger. Une
  // autre copie d'un deck déjà ouvert fait d'abord poser la question ; fusionné
  // dans l'onglet déjà actif, son `App` relit le snapshot (`bump`).
  const onOpen = (path: string) =>
    void run(
      openGuarded(label, path).then(async (notice) => {
        const v = await workspace.view();
        if (notice && alive.current) {
          setMerged(notice);
          bump();
        }
        return v;
      }),
    );
  // Un .pptx importé devient l'onglet actif (toujours un nouveau) : `key` remonte l'`App`, et
  // l'import oublié ferme son dialogue. Un refus va au dialogue, ou ici s'il est déjà fermé.
  const importFile = async (path: string, target: ImportTarget, tune: boolean) => {
    const v = await workspace.importTab(label, path, target, tune);
    if (alive.current) {
      setView(v);
      setError(null);
    }
  };
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
      {update.offer && (
        <UpdateBar
          offer={update.offer}
          installing={update.installing}
          percent={update.percent}
          onInstall={update.install}
          onLater={update.later}
          onOpenRelease={update.openRelease}
        />
      )}
      <div className="workspace-body">
        {/* En développement, `onChange` relit la vue sans relancer le snapshot : pas de boucle. */}
        <App
          key={win.active}
          tab={win.active}
          clip={shared.clip}
          clipGen={shared.gen}
          onCopy={shared.share}
          agentDetached={view.agent_detached}
          nonce={nonce}
          notice={mergeNotice}
          onOpen={onOpen}
          importJob={imports.job?.shown ? imports.job : null}
          onImportPick={imports.pick}
          onImport={(target, tune) => void imports.start({ target, tune }, (path) => importFile(path, target, tune))}
          onImportClose={imports.close}
          onHelp={() => run(workspace.openHelp(label))}
          updates={appUpdates}
          onClose={inTauri || win.tabs.length > 1 ? () => void closeTab(win.active) : undefined}
          onChange={inTauri ? undefined : reread}
        />
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
