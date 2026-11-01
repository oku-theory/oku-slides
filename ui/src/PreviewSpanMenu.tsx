// The preview-slot select of a pack tab's header.

import type { PreviewSpan } from "./api";
import { t } from "./i18n";
import { Select } from "./primitives";

// Literal ids, so the key-usage test sees them.
const SPANS: readonly (readonly [PreviewSpan, string])[] = [
  ["full", "ui-preview-span-full"],
  ["half", "ui-preview-span-half"],
  ["third", "ui-preview-span-third"],
];

/** Picks the share of the grid an element widget's preview spans. */
export default function PreviewSpanMenu({ span, onPick }: { span: PreviewSpan; onPick(span: PreviewSpan): void }) {
  // `Select` takes no className: the wrapper carries the hook for styles and tests.
  return (
    <span className="preview-span-menu">
      <Select
        aria-label={t("ui-preview-span")}
        title={t("ui-preview-span")}
        value={span}
        options={SPANS.map(([value, key]) => ({ value, label: t(key) }))}
        onChange={(v) => onPick(v as PreviewSpan)}
      />
    </span>
  );
}
