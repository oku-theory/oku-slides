// Update bars at the top of the window: a newer version to install (or see the release) or
// put off, its download progress with the buttons disabled; a failed install after restart.

import type { UpdateOffer } from "./api";
import { t } from "./i18n";
import { Button } from "./primitives";

/** Bar offering a newer version of the app. */
export default function UpdateBar({ offer, installing, percent, onInstall, onLater, onOpenRelease }: {
  offer: UpdateOffer;
  installing: boolean;
  /** Download progress, `null` if the size is unknown. */
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

/** Bar telling, after the restart, that an update could not be installed. */
export function UpdateNoticeBar({ text, onDismiss }: { text: string; onDismiss: () => void }) {
  return (
    <div className="notice-bar update-bar" role="alert">
      <span className="notice-bar-text update-bar-text">{text}</span>
      <span className="notice-bar-actions">
        <Button ghost onClick={onDismiss}>{t("ui-update-notice-close")}</Button>
      </span>
    </div>
  );
}
