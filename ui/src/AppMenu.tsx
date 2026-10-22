// Le menu principal d'un onglet : présenter, ouvrir et l'aide. Le reste se demande à l'agent (t-988).

import type { MenuEntry } from "./ActionMenu";
import { ActionMenu } from "./ActionMenu";
import { t } from "./i18n";
import type { PresentMode } from "./present";

/** Ce que les entrées du menu principal déclenchent. */
interface Props {
  /** Mode pack : ni présenter ni ouvrir. */
  packMode: boolean;
  /** Lance la présentation. */
  present: (mode: PresentMode) => void;
  /** Ouvre un fichier. */
  open: () => void;
  /** Ouvre l'aide. */
  onHelp: () => void;
}

/** Le menu principal : ce qu'un geste fait plus vite que la voix, et l'aide. */
export function AppMenu({ packMode, present, open, onHelp }: Props) {
  const help: MenuEntry = { label: t("ui-help"), shortcut: "⌘?", onSelect: onHelp };
  const menuEntries: MenuEntry[] = packMode
    ? [help]
    : [
        { label: t("ui-present-screen"), shortcut: "⌘↩", onSelect: () => present("screen") },
        { label: t("ui-toolbar-open"), shortcut: "⌘O", onSelect: open },
        help,
      ];
  return <ActionMenu label={t("ui-menu")} entries={menuEntries} triggerClass="token ghost" />;
}
