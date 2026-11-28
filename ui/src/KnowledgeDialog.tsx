/**
 * The knowledge dialog: the user reads each received knowledge file, then approves it.
 *
 * It lists the files awaiting approval. Show fetches one file's text (one row open at a time);
 * Approve then sends the hash of the text shown, so a file changed since is refused. Every text
 * is untrusted: a text node in a `<pre>`, hidden characters revealed, never Markdown or HTML.
 *
 * Built on `useModalFrame`: Close has the focus and Escape closes. One `busy` flag, set while an
 * approval runs, disables every button and Escape; a reply to a request no longer current is
 * ignored. After an approval, accepted or refused (`run` says why), the list is fetched once
 * again; an empty list with no ceiling passed closes the dialog.
 */
import { useEffect, useId, useMemo, useRef, useState } from "react";
import { errorText, type KnowledgeReview, type KnowledgeText, type TabApi } from "./api";
import { swallow } from "./ConfirmDialog";
import type { Run } from "./GuidesSection";
import { t } from "./i18n";
import { overLimitText } from "./KnowledgeNotice";
import { revealHidden } from "./knowledgeText";
import type { AwaitingKnowledge } from "./knowledge/types";
import { useModalFrame } from "./modalFrame";
import { Button } from "./primitives";

/** Where the dialog reads and approves, and how it reports and closes. */
interface Props {
  api: TabApi;
  run: Run;
  /** A failed read (list or text), already localized. */
  onError: (message: string) => void;
  onClose: () => void;
}

/** The open row: its id, then its text once it arrived. */
interface Shown {
  id: string;
  text: KnowledgeText | null;
}

/** One file: path, title, size, then its text when shown. */
function Row({ file, shown, busy, onToggle, onApprove }: {
  file: AwaitingKnowledge;
  shown: Shown | null;
  busy: boolean;
  onToggle: () => void;
  onApprove: (text: KnowledgeText) => void;
}) {
  const textId = useId();
  const open = shown?.id === file.id;
  const text = open ? shown.text : null;
  const revealed = useMemo(() => text && revealHidden(text.text), [text]);
  return (
    <li className="knowledge-row">
      <div className="knowledge-head">
        <bdi className="knowledge-path">{revealHidden(file.path).text}</bdi>
        {file.title !== null && (
          <bdi className="knowledge-title">{revealHidden(file.title).text}</bdi>
        )}
        <span className="caption knowledge-size">
          {t("ui-knowledge-size", { kb: Math.ceil(file.bytes / 1024) })}
        </span>
        <Button
          ghost disabled={busy} onClick={onToggle}
          aria-expanded={open} aria-controls={open ? textId : undefined}
        >
          {t(open ? "ui-knowledge-hide" : "ui-knowledge-show")}
        </Button>
      </div>
      {open && !text && (
        <p id={textId} className="caption knowledge-loading">{t("ui-loading")}</p>
      )}
      {text && revealed && (
        <div id={textId} className="knowledge-shown">
          <pre dir="auto" className="guide-text">{revealed.text}</pre>
          {revealed.hidden > 0 && (
            <p className="caption knowledge-hidden">
              {t("ui-knowledge-hidden-chars", { count: revealed.hidden })}
            </p>
          )}
          {text.truncated > 0 ? (
            <p className="caption knowledge-too-large">{t("ui-knowledge-too-large")}</p>
          ) : (
            <Button primary disabled={busy} onClick={() => onApprove(text)}>
              {t("ui-knowledge-approve")}
            </Button>
          )}
        </div>
      )}
    </li>
  );
}

/** Lists the received knowledge files and approves each one read. */
export default function KnowledgeDialog({ api, run, onError, onClose }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const titleId = useId();
  const [review, setReview] = useState<KnowledgeReview | null>(null);
  const [shown, setShown] = useState<Shown | null>(null);
  const [busy, setBusy] = useState(false);
  // The current request of each kind: a reply to an older one is ignored. Bumped on unmount too.
  const listSeq = useRef(0);
  const textSeq = useRef(0);
  // Set once the dialog asked to close or unmounted: no state is set after that.
  const closed = useRef(false);
  const closeButton = useRef<HTMLButtonElement>(null);
  // No Escape while an approval runs: its outcome must still be shown.
  const { onClickCapture, armed } = useModalFrame(ref, { onEscape: busy ? undefined : onClose });

  /** Fetches the list; closes when nothing awaits and no ceiling is passed. */
  const load = async () => {
    const seq = ++listSeq.current;
    try {
      const next = await api.knowledgeReview();
      if (seq !== listSeq.current) return;
      if (next.files.length === 0 && next.more === 0 && next.over_limit.length === 0) {
        closed.current = true;
        onClose();
      } else setReview(next);
    } catch (e) {
      if (seq === listSeq.current) onError(errorText(e));
    }
  };

  // Once per mount: `load` reads only refs and props the parent keeps for the dialog's life (the
  // tab's api and callbacks), so the first render's closure stays right.
  useEffect(() => {
    void load();
    return () => {
      closed.current = true;
      listSeq.current += 1;
      textSeq.current += 1;
    };
  }, []);
  // After the frame took the focus: Close.
  useEffect(() => {
    closeButton.current?.focus({ preventScroll: true });
  }, []);

  const toggle = async (id: string) => {
    const seq = ++textSeq.current;
    if (shown?.id === id) {
      setShown(null);
      return;
    }
    setShown({ id, text: null });
    try {
      const text = await api.knowledgeText(id);
      if (seq === textSeq.current) setShown({ id, text });
    } catch (e) {
      if (seq === textSeq.current) onError(errorText(e));
    }
  };

  const approve = async (text: KnowledgeText) => {
    if (busy) return;
    setBusy(true);
    const seq = ++textSeq.current;
    try {
      await run(api.approveKnowledge([{ id: text.id, hash: text.hash }]));
      if (seq !== textSeq.current) return;
      setShown(null);
      await load();
    } catch (e) {
      // `run` says a refusal itself; anything it lets through is said here.
      if (!closed.current) onError(errorText(e));
    } finally {
      if (!closed.current) setBusy(false);
    }
  };

  return (
    <div className="import-veil">
      <div
        ref={ref}
        tabIndex={-1}
        className="import-dialog confirm-dialog received-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        data-armed={armed || undefined}
        onPointerDown={(e) => e.stopPropagation()}
        onClickCapture={onClickCapture}
        onKeyDown={swallow}
      >
        <h2 id={titleId} className="received-title">{t("ui-knowledge-dialog-title")}</h2>
        <div className="received-body">
          {review && review.over_limit.length > 0 && (
            <p className="confirm-message knowledge-over-limit">
              {overLimitText(review.over_limit)}
            </p>
          )}
          {review && review.files.length > 0 && (
            <ul className="knowledge-files">
              {review.files.map((f) => (
                <Row
                  key={f.id} file={f} shown={shown} busy={busy}
                  onToggle={() => void toggle(f.id)} onApprove={(x) => void approve(x)}
                />
              ))}
            </ul>
          )}
          {review && review.more > 0 && (
            <p className="caption knowledge-more">
              {t("ui-knowledge-more", { count: review.more })}
            </p>
          )}
        </div>
        <div className="import-actions">
          <Button ref={closeButton} ghost className="knowledge-close" disabled={busy} onClick={onClose}>
            {t("ui-knowledge-close")}
          </Button>
        </div>
      </div>
    </div>
  );
}
