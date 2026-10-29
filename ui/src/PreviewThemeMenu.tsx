// The preview-theme select of a pack tab's header.

import { t } from "./i18n";
import type { ThemeInfo } from "./packTypes";
import { Select } from "./primitives";

/** Picks the theme the pack's working deck is previewed in; `theme` is the current namespace. */
export default function PreviewThemeMenu({ themes, theme, onPick }: {
  themes: ThemeInfo[];
  theme: string | null;
  onPick(ns: string): void;
}) {
  return (
    <Select
      aria-label={t("ui-preview-theme")}
      title={t("ui-preview-theme")}
      value={theme ?? ""}
      options={themes.map((th) => ({ value: th.namespace, label: th.name }))}
      onChange={onPick}
    />
  );
}
