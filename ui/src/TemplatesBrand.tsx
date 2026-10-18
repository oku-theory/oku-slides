/** Marque du mode modèles, et la note d'un pack du deck seul (celui d'un import). */
import { t } from "./i18n";

/** Ce que la marque du mode modèles doit dire du pack édité. */
interface TemplatesBrandProps {
  /** Le pack édité n'appartient qu'au deck : ses enregistrements y restent, jamais dans la bibliothèque. */
  deckOnly: boolean;
}

/** La marque « Modèles » de la barre, suivie d'une note quand le pack n'est qu'au deck. */
export function TemplatesBrand({ deckOnly }: TemplatesBrandProps) {
  return (
    <>
      <span className="brand">{t("ui-brand-templates")}</span>
      {deckOnly && <span className="brand-note">{t("ui-templates-deck-only")}</span>}
    </>
  );
}
