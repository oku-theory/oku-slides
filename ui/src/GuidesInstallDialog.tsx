/**
 * The install dialog: a deck brought pack and theme versions whose agent guides the user has not
 * approved. It lists each one with what it holds and both guides as plain text, then asks.
 *
 * Built on the `ConfirmDialog` shell (`useModalFrame`): clicks are ignored for `ARMING_MS`, Enter
 * and digits do nothing, and the tab reports it open so nothing behind it reacts. "Later" has the
 * focus and Escape means Later. Guide text is untrusted: it is a text node in a `<pre>`, never
 * Markdown or HTML.
 */
import { useEffect, useId, useRef, useState } from "react";
import type { GuidesReview, TabApi, VersionRef } from "./api";
import { swallow } from "./ConfirmDialog";
import type { Run } from "./GuidesSection";
import { isolate, t } from "./i18n";
import { useModalFrame } from "./modalFrame";
import { Button } from "./primitives";

/** What the dialog asks about and how it answers. */
interface Props {
  api: TabApi;
  reviews: GuidesReview[];
  run: Run;
}

/** The name of each kind of version (literal ids, so unused messages are caught). */
const KIND = {
  widgets: "ui-guides-kind-widgets",
  layouts: "ui-guides-kind-layouts",
  theme: "ui-guides-kind-theme",
} as const;

/** One version: name, kind, size, requirements, then its guides. */
function Review({ review }: { review: GuidesReview }) {
  const { guides } = review;
  return (
    <section className="guides-review">
      <h3 className="guides-review-name">{review.name}</h3>
      <p className="caption">
        {[
          t(KIND[review.kind]),
          t("ui-guides-install-items", { count: review.items }),
          ...(review.requires.length > 0 ? [t("ui-guides-install-requires", { names: review.requires.map(isolate).join(", ") })] : []),
        ].join(" · ")}
      </p>
      {guides.edit && (
        <>
          <p className="caption">{t("ui-guides-edit")}</p>
          <pre className="guide-text">{guides.edit}</pre>
        </>
      )}
      {guides.use && (
        <>
          <p className="caption">{t("ui-guides-use")}</p>
          <pre className="guide-text">{guides.use}</pre>
        </>
      )}
    </section>
  );
}

/** Asks whether the guides the open deck brings may reach the agent. */
export default function GuidesInstallDialog({ api, reviews, run }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const titleId = useId();
  const [busy, setBusy] = useState(false);
  const versions: VersionRef[] = reviews.map((r) => ({ id: r.id, version: r.version }));
  const answer = (ask: (v: VersionRef[]) => ReturnType<TabApi["approveGuides"]>) => {
    if (busy) return;
    setBusy(true);
    // Answered: the snapshot no longer lists them and the dialog goes. Refused: it stays to try again.
    void run(ask(versions)).then(() => setBusy(false));
  };
  const onLater = () => answer((v) => api.laterGuides(v));
  const { onClickCapture, armed } = useModalFrame(ref, { onEscape: onLater });
  // After the frame took the focus: Later, never the approval.
  useEffect(() => ref.current?.querySelector<HTMLButtonElement>(".confirm-cancel")?.focus(), []);
  return (
    <div className="import-veil">
      <div
        ref={ref}
        tabIndex={-1}
        className="import-dialog confirm-dialog guides-install"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby={titleId}
        data-armed={armed || undefined}
        onPointerDown={(e) => e.stopPropagation()}
        onClickCapture={onClickCapture}
        onKeyDown={swallow}
      >
        <h2 id={titleId} className="guides-install-title">{t("ui-guides-install-title")}</h2>
        <p className="confirm-message">{t("ui-guides-install-intro")}</p>
        {reviews.map((r) => <Review key={`${r.id}/${r.version}`} review={r} />)}
        <div className="import-actions">
          <Button ghost className="confirm-cancel" disabled={busy} onClick={onLater}>{t("ui-guides-install-later")}</Button>
          <Button primary className="confirm-ok" disabled={busy} onClick={() => answer((v) => api.approveGuides(v))}>
            {t("ui-guides-install-approve")}
          </Button>
        </div>
      </div>
    </div>
  );
}
