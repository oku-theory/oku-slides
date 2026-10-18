// Bandeau de mise à jour, en haut de la fenêtre : une version plus récente,
// l'installer (ou voir la release), ou plus tard ; pendant le téléchargement,
// son avancée, boutons désactivés.

import type { UpdateOffer } from "./api";
import { t } from "./i18n";
import { Button } from "./primitives";

/** Bandeau proposant une version plus récente de l'app. */
export default function UpdateBar({ offer, installing, percent, onInstall, onLater, onOpenRelease }: {
  offer: UpdateOffer;
  installing: boolean;
  /** Avancée du téléchargement, `null` si la taille est inconnue. */
  percent: number | null;
  onInstall: () => void;
  onLater: () => void;
  onOpenRelease: () => void;
}) {
  const text = !installing
    ? t("ui-update-available", { version: offer.version })
    : percent === null ? t("ui-update-downloading") : t("ui-update-installing", { percent });
  return (
    <div className="notice-bar update-bar" role="status">
      <span className="notice-bar-text update-bar-text">{text}</span>
      {installing && (percent === null
        ? <progress className="update-bar-progress" max={100} />
        : <progress className="update-bar-progress" max={100} value={percent} />)}
      <span className="notice-bar-actions">
        {offer.mode === "install" ? (
          <Button primary disabled={installing} onClick={onInstall}>{t("ui-update-install")}</Button>
        ) : (
          <Button primary disabled={installing} onClick={onOpenRelease}>{t("ui-update-see-release")}</Button>
        )}
        <Button ghost disabled={installing} onClick={onLater}>{t("ui-update-later")}</Button>
      </span>
    </div>
  );
}
