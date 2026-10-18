// Le menu principal d'un onglet : présenter, fichiers, atelier, thème, schéma, langue et aide.

import type { MenuEntry } from "./ActionMenu";
import { ActionMenu } from "./ActionMenu";
import type { Snapshot, TabApi } from "./api";
import { errorText } from "./api";
import { availableLocales, changeLocale, chosenLocale, languageName, t } from "./i18n";
import type { PresentMode } from "./present";
import { Field, Select } from "./primitives";
import { changeScheme, type useSchemeChoice } from "./scheme";
import SchemePicker from "./SchemePicker";
import ThemeMenu from "./ThemeMenu";
import type { KeptUi } from "./useKeptUi";

/** Ce que le menu principal lit et ce que ses entrées déclenchent. */
interface Props {
  /** Requêtes de l'onglet : le thème du deck. */
  api: TabApi;
  /** Le snapshot : thèmes, et mode modèles. */
  snap: Snapshot;
  /** Le schéma de couleurs choisi. */
  schemeChoice: ReturnType<typeof useSchemeChoice>;
  /** Les réglages d'interface gardés par l'hôte, et leurs entrées. */
  keptUi: KeptUi;
  /** Applique une requête qui rend un snapshot. */
  run: (p: Promise<Snapshot | null>) => Promise<Snapshot | null>;
  /** Affiche un message, une erreur si `error`. */
  say: (text: string, error?: boolean) => void;
  /** Lance la présentation. */
  present: (mode: PresentMode) => void;
  /** Ouvre un fichier. */
  open: () => void;
  /** Fusionne un autre deck dans celui-ci. */
  merge: () => void;
  /** Déplace le fichier du deck. */
  moveFile: () => void;
  /** Enregistre une copie du deck. */
  saveCopy: () => void;
  /** Entre dans l'atelier des modèles. */
  editTemplates: () => Promise<boolean>;
  /** Ouvre l'historique. */
  openHistory: () => void;
  /** Enregistre le pack édité, comme un nouveau si `asNew`. */
  saveTemplates: (asNew?: boolean) => void;
  /** Ouvre l'aide. */
  onHelp: () => void;
  /** Cherche une mise à jour de l'app ; absent hors de l'app (pas d'entrée). */
  onCheckUpdates?: () => void;
}

/** Le menu principal ; ses entrées changent en mode modèles. */
export function AppMenu({
  api, snap, schemeChoice, keptUi, run, say, present, open, merge, moveFile, saveCopy, editTemplates,
  openHistory, saveTemplates, onHelp, onCheckUpdates,
}: Props) {
  const checkUpdates: MenuEntry[] = onCheckUpdates
    ? [{ kind: "item", label: t("ui-menu-check-updates"), onSelect: onCheckUpdates }]
    : [];
  const themePicker = (
    <ThemeMenu
      theme={snap.theme}
      themeName={snap.theme_name}
      themes={snap.themes}
      deckThemes={snap.deck_themes}
      onPick={(id) => run(api.setTheme(id))}
    />
  );
  const languagePicker = (
    <Field variant="menu" label={t("ui-language")}>
      <Select
        value={chosenLocale() ?? ""}
        onChange={(v) => changeLocale(v || null).catch((err) => say(errorText(err), true))}
        options={[{ value: "", label: t("ui-language-system") }, ...availableLocales().map((code) => ({ value: code, label: languageName(code) }))]}
      />
    </Field>
  );
  const schemePicker = (
    <SchemePicker value={schemeChoice} onChange={(c) => changeScheme(c).catch((err) => say(errorText(err), true))} />
  );
  const menuEntries: MenuEntry[] = snap.editing
    ? [
        { kind: "item" as const, label: t("ui-toolbar-save-copy"), onSelect: () => saveTemplates(true) },
        { kind: "separator" },
        ...keptUi.entries,
        { kind: "separator" },
        { kind: "custom", node: schemePicker },
        { kind: "custom", node: languagePicker },
        { kind: "separator" },
        ...checkUpdates,
        { kind: "item", label: t("ui-help"), shortcut: "⌘?", onSelect: onHelp },
      ]
    : [
        { kind: "item", label: t("ui-present-screen"), shortcut: "⌘↩", onSelect: () => present("screen") },
        { kind: "item", label: t("ui-present-window"), shortcut: "⌘⇧↩", onSelect: () => present("window") },
        { kind: "separator" },
        { kind: "item", label: t("ui-toolbar-open"), shortcut: "⌘O", onSelect: open },
        { kind: "item", label: t("ui-merge"), onSelect: merge },
        { kind: "separator" },
        { kind: "item", label: t("ui-menu-move"), onSelect: moveFile },
        { kind: "item", label: t("ui-toolbar-save-copy"), shortcut: "⌘⇧S", onSelect: saveCopy },
        { kind: "separator" },
        { kind: "item", label: t("ui-menu-workshop"), onSelect: () => void editTemplates() },
        { kind: "item", label: t("ui-history"), onSelect: openHistory },
        { kind: "custom", node: themePicker },
        ...keptUi.entries,
        { kind: "separator" },
        { kind: "custom", node: schemePicker },
        { kind: "custom", node: languagePicker },
        { kind: "separator" },
        ...checkUpdates,
        { kind: "item", label: t("ui-help"), shortcut: "⌘?", onSelect: onHelp },
      ];
  return <ActionMenu label={t("ui-menu")} entries={menuEntries} triggerClass="token ghost" />;
}
