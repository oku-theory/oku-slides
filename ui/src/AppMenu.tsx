// The main menu of a tab: present, open, new packs and themes, updates, help and About.
// Everything else is asked of the agent (t-988).

import type { MenuEntry } from "./ActionMenu";
import { ActionMenu } from "./ActionMenu";
import { t } from "./i18n";
import type { PackKind } from "./packTypes";
import type { PresentMode } from "./present";
import type { TabKind } from "./tabKind";

/** What the main menu entries trigger. */
interface Props {
  /** The tab's kind: it sets which entries show (decks present and open, themes only create themes). */
  kind: TabKind;
  /** Starts the presentation. */
  present: (mode: PresentMode) => void;
  /** Opens a file. */
  open: () => void;
  /** Opens the help. */
  onHelp: () => void;
  /** Opens the About dialog (version and changelog). */
  onAbout: () => void;
  /** Checks for an update; absent outside the app (no host). */
  onCheckUpdates?: () => void;
  /** Creates an empty pack of `kind` in its own tab. */
  onNewPack: (kind: PackKind) => void;
  /** Creates an empty theme in its own tab. */
  onNewTheme: () => void;
}

/** The main menu: quick gestures, pack and theme creation, updates and help. */
export function AppMenu({ kind, present, open, onHelp, onAbout, onCheckUpdates, onNewPack, onNewTheme }: Props) {
  const app: MenuEntry[] = [
    ...(onCheckUpdates ? [{ label: t("ui-update-check"), onSelect: onCheckUpdates }] : []),
    { label: t("ui-help"), shortcut: "⌘?", onSelect: onHelp },
    { label: t("ui-about"), onSelect: onAbout },
  ];
  const newTheme: MenuEntry = { label: t("ui-menu-new-theme"), onSelect: onNewTheme };
  const newPacks: MenuEntry[] = [
    { label: t("ui-menu-new-widget-pack"), onSelect: () => onNewPack("widgets") },
    { label: t("ui-menu-new-layout-pack"), onSelect: () => onNewPack("layouts") },
  ];
  const deck: MenuEntry[] = [
    { label: t("ui-present-screen"), shortcut: "⌘↩", onSelect: () => present("screen") },
    { label: t("ui-toolbar-open"), shortcut: "⌘O", onSelect: open },
  ];
  const creating = kind === "deck" || kind === "pack" ? [...newPacks, newTheme] : kind === "theme" ? [newTheme] : [];
  const menuEntries: MenuEntry[] = [...(kind === "deck" ? deck : []), ...creating, ...app];
  return <ActionMenu label={t("ui-menu")} entries={menuEntries} triggerClass="token ghost" />;
}
