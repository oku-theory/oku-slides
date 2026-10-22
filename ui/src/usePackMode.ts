// Le mode pack d'un onglet : y entrer, en sortir. L'enregistrer se demande à l'agent (t-988).

import type { Snapshot, TabApi } from "./api";
import { t } from "./i18n";
import { firstPackId } from "./packOps";
import type { useFileActions } from "./useFileActions";
import type { PackWidgetEditor } from "./usePackWidgetEditor";

/** Ce dont le mode pack a besoin de l'onglet. */
interface PackModeContext {
  api: TabApi;
  snap: Snapshot | null;
  snapRef: { current: Snapshot | null };
  run: (p: Promise<Snapshot | null>) => Promise<Snapshot | null>;
  autosave: ReturnType<typeof useFileActions>["autosave"];
  widgets: PackWidgetEditor;
  setCurrent: (id: string | null) => void;
  setSelected: (id: string | null) => void;
}

/** Les gestes du mode pack : éditer un pack, le quitter. */
export function usePackMode({ api, snap, snapRef, run, autosave, widgets, setCurrent, setSelected }: PackModeContext) {
  // Un pack du deck seul (import) s'y édite aussi : ses enregistrements restent dans le deck.
  // `pack` : l'id du pack à éditer ; omis, le pack par défaut ; nul, aucun (rien n'est ouvert).
  const editPack = async (pack: string | null = firstPackId(snap)): Promise<boolean> => {
    if (!pack) return false;
    if (!(await run(api.editPack(pack)))) return false;
    setCurrent(null);
    setSelected(null);
    return true;
  };
  const finishPack = async () => {
    if (!snap?.editing) return;
    await autosave.flush();
    // Un enregistrement refusé laisse un pack non enregistré.
    if (snapRef.current?.dirty && !window.confirm(t("ui-confirm-quit-pack"))) return;
    let closed = false;
    await widgets.finish(async () => { closed = !!(await run(api.closePack())); });
    if (closed) {
      setCurrent(null);
      setSelected(null);
    }
  };
  return { editPack, finishPack };
}
