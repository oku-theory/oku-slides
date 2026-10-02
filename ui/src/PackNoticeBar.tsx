// Bandeau du pack : le deck embarque une autre version que ma version
// courante. Textes et libellés viennent de la session, dans la langue de
// l'utilisateur ; il ne bloque rien et se ferme pour la session.

import type { PackNotice } from "./api";
import { t } from "./i18n";
import { Icon } from "./icons";

export default function PackNoticeBar({ notice, onUseMine, onMakeCurrent, onClose }: {
  notice: PackNotice;
  /** Passer le deck à ma version courante (annulable). */
  onUseMine: () => void;
  /** Faire de la sienne ma version courante (le deck ne change pas) ; masqué sans libellé. */
  onMakeCurrent: () => void;
  onClose: () => void;
}) {
  const close = t("ui-pack-notice-close");
  return (
    <div className="object pack-notice" role="status">
      <span className="pack-notice-text">{notice.message}</span>
      <span className="pack-notice-actions">
        <button className="token" onClick={onUseMine}>{notice.use_mine}</button>
        {notice.make_current !== null && (
          <button className="token ghost" onClick={onMakeCurrent}>{notice.make_current}</button>
        )}
      </span>
      <button className="token ghost pack-notice-close" aria-label={close} title={close} onClick={onClose}>{Icon.close}</button>
    </div>
  );
}
