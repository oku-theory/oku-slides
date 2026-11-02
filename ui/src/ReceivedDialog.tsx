/**
 * The received dialog: one validation for the packs and themes an open deck brings.
 *
 * It lists every version the library lacks, or whose guides await approval, with its capability
 * manifest, why it would stay in the deck (`blocker`) and its guides, then asks. The primary
 * button adds the installable versions to the library and approves the guides of all listed;
 * when none installs it only approves guides, and when nothing installs nor awaits approval it
 * reads OK (the host sets the blocked versions aside). "Later" sets them aside for the tab.
 *
 * Built on the `ConfirmDialog` shell (`useModalFrame`): clicks are ignored for `ARMING_MS`, Enter
 * and digits do nothing, and the tab reports it open so nothing behind it reacts. A changed list
 * re-mounts the frame, so it arms again. "Later" has the focus and Escape means Later. Every received text is untrusted: text nodes only, guides in a
 * `<pre>`, never Markdown or HTML.
 */
import { useEffect, useId, useRef, useState } from "react";
import type { ReceivedReview, Snapshot, TabApi, VersionRef } from "./api";
import { swallow } from "./ConfirmDialog";
import type { Run } from "./GuidesSection";
import { isolate, t } from "./i18n";
import { useModalFrame } from "./modalFrame";
import { Button } from "./primitives";

/** What the dialog asks about and how it answers. */
interface Props {
  api: TabApi;
  reviews: ReceivedReview[];
  run: Run;
}

/** The name of each kind of version, as literal message ids. */
const KIND = {
  widgets: "ui-received-kind-widgets",
  layouts: "ui-received-kind-layouts",
  theme: "ui-received-kind-theme",
} as const;

/** How each version relates to the library. */
const RELATION = {
  new_pack: "ui-received-relation-new-pack",
  new_version: "ui-received-relation-new-version",
  held: "ui-received-relation-held",
} as const;

/** The manifest's lines, each shown only when it says something. */
function fields(review: ReceivedReview): string[] {
  const { capabilities: c, totals } = review;
  const list = (key: string, names: string[], total: number) => {
    if (names.length === 0) return [];
    const shown = names.map(isolate).join(", ");
    const more = total - names.length;
    return [t(key, { names: more > 0 ? t("ui-received-more", { names: shown, count: more }) : shown })];
  };
  return [
    ...list("ui-received-items", c.items, totals.items),
    ...list("ui-received-requires", c.requires, totals.requires),
    ...list("ui-received-tokens", c.tokens, totals.tokens),
    ...list("ui-received-fonts", c.fonts, totals.fonts),
    ...(c.profiles > 0 ? [t("ui-received-profiles", { count: c.profiles })] : []),
    ...(c.code_bytes > 0 ? [t("ui-received-code", { kb: Math.ceil(c.code_bytes / 1024) })] : []),
  ];
}

/** One version: name, namespace, kind, manifest, fate, then its guides. */
function Review({ review }: { review: ReceivedReview }) {
  const { capabilities: c, guides } = review;
  return (
    <section className="received-review">
      <h3 className="received-name">{c.name}</h3>
      <p className="caption received-namespace">{t("ui-received-namespace", { namespace: c.namespace })}</p>
      <p className="caption received-kind">{t("ui-received-kind-line", { kind: t(KIND[c.kind]), relation: t(RELATION[review.relation]) })}</p>
      {c.description && <p className="received-description">{c.description}</p>}
      {fields(review).map((line) => <p key={line} className="caption received-field">{line}</p>)}
      {review.blocker !== null && <p className="caption received-blocker">{review.blocker}</p>}
      {review.install && <p className="caption received-install">{t("ui-received-install")}</p>}
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

/** The primary button's label: add, approve guides, or a plain OK. */
function primaryLabel(reviews: ReceivedReview[]): string {
  if (reviews.some((r) => r.install)) return t("ui-received-add");
  if (reviews.some((r) => r.guides.edit || r.guides.use)) return t("ui-received-approve");
  return t("ui-received-dismiss");
}

/** Asks whether what the open deck brings may enter the library. */
export default function ReceivedDialog(props: Props) {
  // Another version joining (or leaving) re-mounts the frame: it arms again and refocuses Later.
  const key = props.reviews.map((r) => `${r.id}/${r.version}`).join("\n");
  return <Frame key={key} {...props} />;
}

/** The dialog itself, mounted once per list of versions. */
function Frame({ api, reviews, run }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const titleId = useId();
  const introId = useId();
  const [busy, setBusy] = useState(false);
  const versions: VersionRef[] = reviews.map((r) => ({ id: r.id, version: r.version }));
  const answer = (ask: (v: VersionRef[]) => Promise<Snapshot>) => {
    if (busy) return;
    setBusy(true);
    // Answered: the snapshot no longer lists them and the dialog goes. Refused: it stays to try again.
    void run(ask(versions)).then(() => setBusy(false));
  };
  const onLater = () => answer((v) => api.laterReceived(v));
  const { onClickCapture, armed } = useModalFrame(ref, { onEscape: onLater });
  // After the frame took the focus: Later, never the validation.
  // No scroll: the actions row stays pinned below the scrolling body, which keeps its place.
  useEffect(() => ref.current?.querySelector<HTMLButtonElement>(".confirm-cancel")?.focus({ preventScroll: true }), []);
  return (
    <div className="import-veil">
      <div
        ref={ref}
        tabIndex={-1}
        className="import-dialog confirm-dialog received-dialog"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={introId}
        data-armed={armed || undefined}
        onPointerDown={(e) => e.stopPropagation()}
        onClickCapture={onClickCapture}
        onKeyDown={swallow}
      >
        <h2 id={titleId} className="received-title">{t("ui-received-title")}</h2>
        <div className="received-body">
          <p id={introId} className="confirm-message">{t("ui-received-intro")}</p>
          {reviews.map((r) => <Review key={`${r.id}/${r.version}`} review={r} />)}
          <p className="caption received-offline">{t("ui-received-offline")}</p>
        </div>
        <div className="import-actions">
          <Button ghost className="confirm-cancel" disabled={busy} onClick={onLater}>{t("ui-received-later")}</Button>
          <Button primary className="confirm-ok" disabled={busy} onClick={() => answer((v) => api.acceptReceived(v))}>
            {primaryLabel(reviews)}
          </Button>
        </div>
      </div>
    </div>
  );
}
