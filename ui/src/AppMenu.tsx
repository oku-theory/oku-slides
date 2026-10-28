// Le menu principal d'un onglet : présenter, ouvrir, les mises à jour, l'aide et « À propos ».
// Le reste se demande à l'agent (t-988).

import type { MenuEntry } from "./ActionMenu";
import { ActionMenu } from "./ActionMenu";
import { t } from "./i18n";
import type { PresentMode } from "./present";

/** Ce que les entrées du menu principal déclenchent. */
interface Props {
  /** Mode pack : ni présenter ni ouvrir. */
  packMode: boolean;
  /** A theme tab: no deck entries either (present, open). */
  themeTab: boolean;
  /** Lance la présentation. */
  present: (mode: PresentMode) => void;
  /** Ouvre un fichier. */
  open: () => void;
  /** Ouvre l'aide. */
  onHelp: () => void;
  /** Opens the About dialog (version and changelog). */
  onAbout: () => void;
  /** Cherche une mise à jour ; absent hors de l'app (pas d'hôte). */
  onCheckUpdates?: () => void;
}

/** Le menu principal : gestes plus rapides que la voix, mises à jour et aide. */
export function AppMenu({ packMode, themeTab, present, open, onHelp, onAbout, onCheckUpdates }: Props) {
  const app: MenuEntry[] = [
    ...(onCheckUpdates ? [{ label: t("ui-update-check"), onSelect: onCheckUpdates }] : []),
    { label: t("ui-help"), shortcut: "⌘?", onSelect: onHelp },
    { label: t("ui-about"), onSelect: onAbout },
  ];
  const menuEntries: MenuEntry[] = packMode || themeTab
    ? app
    : [
        { label: t("ui-present-screen"), shortcut: "⌘↩", onSelect: () => present("screen") },
        { label: t("ui-toolbar-open"), shortcut: "⌘O", onSelect: open },
        ...app,
      ];
  return <ActionMenu label={t("ui-menu")} entries={menuEntries} triggerClass="token ghost" />;
}
