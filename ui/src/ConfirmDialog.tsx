// The shared confirmation dialog: a question, a confirming button and Cancel.
//
// Built on `useModalFrame`: clicks are ignored for `ARMING_MS`, Escape cancels, and neither
// Enter nor a digit does anything, so a keystroke already under way never confirms.

import { useId, useRef, type KeyboardEvent } from "react";
import { t } from "./i18n";
import { useModalFrame } from "./modalFrame";
import { Button } from "./primitives";

/** The question asked and its two answers. */
export interface ConfirmDialogProps {
  /** The question, already localized. */
  message: string;
  /** The confirming button's label, already localized. */
  confirmLabel: string;
  onConfirm(): void;
  onCancel(): void;
}

/** Enter and digits never act in a confirmation, nor reach the app behind it. */
function swallow(e: KeyboardEvent) {
  if (e.key !== "Enter" && !/^Digit|^Numpad\d/.test(e.code) && !/^\d$/.test(e.key)) return;
  e.preventDefault();
  e.stopPropagation();
}

/** Asks `message`; confirming and cancelling are both armed after `ARMING_MS`. */
export default function ConfirmDialog({ message, confirmLabel, onConfirm, onCancel }: ConfirmDialogProps) {
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
        <p id={messageId} className="confirm-message">{message}</p>
        <div className="import-actions">
          <Button ghost className="confirm-cancel" onClick={onCancel}>{t("ui-close-cancel")}</Button>
          <Button danger className="confirm-ok" onClick={onConfirm}>{confirmLabel}</Button>
        </div>
      </div>
    </div>
  );
}
