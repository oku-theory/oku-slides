// Une fenêtre : l'`App` de l'onglet actif, la barre d'onglets en bas.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import AboutDialog from "./AboutDialog";
import App from "./App";
import { APP_VERSION, RELEASES } from "./appInfo";
import TabBar from "./TabBar";
import UpdateBar from "./UpdateBar";
import { closeStays, errorText, inTauri, isMac, listenTauri, tabApi, tabDrag, tauriUpdates, workspace, type ImportTarget, type WorkspaceView } from "./api";
import { closeFlowDeps, closeTabFlow, closeWindowFlow } from "./closeFlow";
import { runCloseGuard } from "./closeGuards";
import { t } from "./i18n";
import { newest } from "./latest";
import { mergedNotice, openGuarded } from "./merge";
import { cycle, nth, othersToClose } from "./tabs";
import { useImportJob } from "./useImportJob";
import { useImportPick } from "./useImportPick";
import { useSharedClip } from "./useSharedClip";
import { useTabShortcuts } from "./useTabShortcuts";
import { ERROR_MS } from "./useToast";
import { useConfirm } from "./useConfirm";
import { useTransient } from "./useTransient";
import { useUpdate } from "./useUpdate";
import { showBroadcast } from "./workspaceBroadcast";

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

  // Every view the host broadcasts makes the `App` re-read its snapshot (showBroadcast): a
  // notice set on the active tab, or an owner deck changed by its deck-pack tab, shows at once.
  const [nonce, setNonce] = useState(0);
  const bump = useCallback(() => setNonce((n) => n + 1), []);
  // The About dialog, one per window (main menu of any tab).
  const [about, setAbout] = useState(false);
  // The close flows' questions, one at a time over the active tab (t-1254).
  const asker = useConfirm();
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

  /** Applies a workspace request's view; null on failure, the error shown and the view reread. */
  const run = useCallback(
    async (p: Promise<WorkspaceView>): Promise<WorkspaceView | null> => {
      try {
        const v = await p;
        if (alive.current) {
          setView(v);
          setError(null);
        }
        return v;
      } catch (e) {
        if (alive.current) setError(errorText(e));
        refresh();
        return null;
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
      const stop = await listen<WorkspaceView>("workspace", (e) => showBroadcast(e.payload, { alive, setView, bump }));
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

  /** The view read right before asking: its "modified" flags are current. */
  const freshView = async () => {
    const v = await workspace.view();
    if (alive.current) setView(v);
    return v;
  };

  // Ids whose close runs (tab and window flows alike): a double click on × asks once.
  const closingRef = useRef(new Set<string>());
  // First stall time per tab: a second one within 30 s asks to close anyway.
  const stallsRef = useRef(new Map<string, number>());

  /** The close flows' deps, built from this render's values for window `l`. */
  const flowDeps = (l: string) =>
    closeFlowDeps({
      tabApi, confirm: asker.confirm, runGuard: (x) => runCloseGuard(x), closing: closingRef.current,
      stalls: stallsRef.current, refresh, freshView, now: Date.now, ws: workspace, label: l, alive, run, setError,
    });

  /** Closes a tab through closeTabFlow (guard, save, offer); false: not closed. */
  const closeTab = useCallback(
    async (id: string): Promise<boolean> => {
      if (label === null) return false;
      return closeTabFlow(id, flowDeps(label));
    },
    [run, label],
  );

  // « Annuler » sur un onglet arrête tout. A deck-pack tab keeps its owner open.
  const closeOthers = async (id: string) => {
    for (const other of othersToClose(winRef.current?.tabs ?? [], id)) {
      if (!(await closeTab(other))) return;
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
        try {
          // Quit runs no guard (t-1206).
          if ((viewRef.current?.windows.length ?? 0) <= 1) return void (await invoke("quit"));
          // The tab active at the request: shown again if the user gives up.
          await closeWindowFlow(winRef.current?.active ?? null, flowDeps(label));
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

  // Tab shortcuts, held while the import dialog, About or a close question is open.
  useTabShortcuts(label !== null, {
    mac: isMac,
    windows: inTauri,
    held: () => imports.holdsKeys() || about || asker.asking,
    onAction: (action) => {
      const w = winRef.current;
      if (!w || label === null) return;
      const ids = w.tabs.map((x) => x.id);
      if (action.kind === "new-tab") run(workspace.newTab(label));
      else if (action.kind === "help") run(workspace.openHelp(label));
      else if (action.kind === "close-tab") {
        const v = viewRef.current;
        if (v && !closeStays(v, label, w.active, inTauri)) closeTab(w.active);
      } else if (action.kind === "new-window") run(workspace.newWindow());
      else {
        const id = action.kind === "cycle" ? cycle(ids, w.active, action.delta) : nth(ids, action.n);
        if (id && id !== w.active) run(workspace.activate(id));
      }
    },
  });

  if (!view || label === null || !win) {
    // A close question asked meanwhile still shows (and can be answered).
    return (
      <div className="table">
        <p className="empty" style={{ padding: 40 }}>{error ?? t("ui-loading")}</p>
        {asker.dialog}
      </div>
    );
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
          onAbout={() => setAbout(true)}
          windowDialog={about || asker.asking}
          updates={appUpdates}
          onClose={closeStays(view, label, win.active, inTauri) ? undefined : () => void closeTab(win.active)}
          onOpenPack={async (pack, widget) => {
            const v = await run(workspace.openPack(win.active, pack, widget));
            return v?.windows.find((w) => w.label === label)?.active === win.active;
          }}
          onOpenTheme={(theme) => void run(workspace.openTheme(win.active, theme))}
          onNewPack={(kind) => void run(workspace.newPack(win.active, kind))}
          onNewTheme={() => void run(workspace.newTheme(win.active))}
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
      {about && <AboutDialog version={APP_VERSION} releases={RELEASES} onClose={() => setAbout(false)} />}
      {asker.dialog}
    </div>
  );
}
