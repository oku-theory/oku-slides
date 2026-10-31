// The shared confirmation dialog: a question, a confirming button and Cancel.
//
// Built on `useModalFrame`: clicks are ignored for `ARMING_MS`, Escape cancels, and neither
// Enter nor a digit does anything, so a keystroke already under way never confirms.

import { useId, useRef, type KeyboardEvent } from "react";
import { t } from "./i18n";
import { useModalFrame } from "./modalFrame";
import { Button } from "./primitives";
import type { Question } from "./useConfirm";

/** The question asked and its two answers. */
export interface ConfirmDialogProps {
  question: Question;
  onConfirm(): void;
  onCancel(): void;
}

/** Enter and digits never act in a confirmation, nor reach the app behind it. */
function swallow(e: KeyboardEvent) {
  if (e.key !== "Enter" && !/^Digit|^Numpad\d/.test(e.code) && !/^\d$/.test(e.key)) return;
  e.preventDefault();
  e.stopPropagation();
}

/** Asks `question`; confirming and cancelling are both armed after `ARMING_MS`. */
export default function ConfirmDialog({ question, onConfirm, onCancel }: ConfirmDialogProps) {
  const ref = useRef<HTMLDivElement>(null);
  const messageId = useId();
  const { onClickCapture, armed } = useModalFrame(ref, { onEscape: onCancel });
  return (
    <div className="import-veil">
      <div
        ref={ref}
        tabIndex={-1}
        className="import-dialog confirm-dialog"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby={messageId}
        aria-describedby={messageId}
        data-armed={armed || undefined}
        onPointerDown={(e) => e.stopPropagation()}
        onClickCapture={onClickCapture}
        onKeyDown={swallow}
      >
        <p id={messageId} className="confirm-message">{question.message}</p>
        <div className="import-actions">
          <Button ghost className="confirm-cancel" onClick={onCancel}>{question.cancel ?? t("ui-close-cancel")}</Button>
          <Button danger={question.danger} primary={!question.danger} className="confirm-ok" onClick={onConfirm}>{question.ok}</Button>
        </div>
      </div>
    </div>
  );
}
