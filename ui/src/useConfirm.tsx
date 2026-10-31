// The shared yes/no question: asked through a promise, answered in a `ConfirmDialog`.
//
// Questions queue: the first is shown, the next once it is answered (each armed afresh). The
// queue lives in a ref, so a question asked right before an unmount is still answered: no, as
// is one asked after it.

import { useCallback, useEffect, useReducer, useRef, type ReactNode } from "react";
import ConfirmDialog from "./ConfirmDialog";

/** A question and the labels of its two answers, all already localized. */
export interface Question {
  message: string;
  /** The confirming button's label. */
  ok: string;
  /** The confirming button loses work (a delete, a close without saving). */
  danger: boolean;
  /** The cancelling button's label; "Cancel" when absent. */
  cancel?: string;
}

/** Asks a question; true when confirmed. */
export type Confirm = (question: Question) => Promise<boolean>;

/** A question waiting for its answer. */
interface Asked {
  id: number;
  question: Question;
  answer(yes: boolean): void;
}

/** What useConfirm gives its component. */
export interface Asker {
  confirm: Confirm;
  /** A question is shown. */
  asking: boolean;
  /** The dialog of the first question, to render; null when none waits. */
  dialog: ReactNode;
}

/** A queue of questions answered one at a time in a `ConfirmDialog`. */
export function useConfirm(): Asker {
  const pending = useRef<Asked[]>([]);
  const next = useRef(0);
  // False once unmounted (true again on StrictMode's second mount).
  const alive = useRef(true);
  const [, rerender] = useReducer((n: number) => n + 1, 0);
  const confirm = useCallback<Confirm>(
    (question) =>
      new Promise<boolean>((resolve) => {
        if (!alive.current) return resolve(false);
        const asked: Asked = {
          id: next.current++,
          question,
          answer: (yes) => {
            pending.current = pending.current.filter((a) => a !== asked);
            rerender();
            resolve(yes);
          },
        };
        pending.current = [...pending.current, asked];
        rerender();
      }),
    [],
  );
  // Questions left when the component goes are answered no.
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      const left = pending.current;
      pending.current = [];
      for (const a of left) a.answer(false);
    };
  }, []);
  const first = pending.current[0];
  const dialog = first ? (
    <ConfirmDialog
      key={first.id}
      question={first.question}
      onConfirm={() => first.answer(true)}
      onCancel={() => first.answer(false)}
    />
  ) : null;
  return { confirm, asking: first !== undefined, dialog };
}
