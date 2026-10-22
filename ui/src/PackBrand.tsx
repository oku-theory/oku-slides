/** Marque du mode pack : le nom et la sorte du pack édité, et la note d'un pack du deck seul (celui d'un import). */
import type { PackKind } from "./api";
import { t } from "./i18n";

/** Ce que la marque du mode pack doit dire du pack édité. */
interface PackBrandProps {
  /** Nom du pack édité, tel qu'enregistré. */
  name: string;
  /** Sorte du pack édité. */
  kind: PackKind;
  /** Le pack édité n'appartient qu'au deck : ses enregistrements y restent, jamais dans la bibliothèque. */
  deckOnly: boolean;
}

/** La marque de la barre (nom, pack de layouts ou de widgets), suivie d'une note quand le pack n'est qu'au deck. */
export function PackBrand({ name, kind, deckOnly }: PackBrandProps) {
  return (
    <>
      <span className="brand">{t("ui-brand-pack", { name, kind })}</span>
      {deckOnly && <span className="brand-note">{t("ui-pack-deck-only")}</span>}
    </>
  );
}
