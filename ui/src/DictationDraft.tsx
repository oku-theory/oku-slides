// The dictation draft in the right panel: the user rereads, edits or completes what they said,
// then sends it to the agent as an instruction. It reads as a card; a click opens it for editing.
// Opened from the keyboard (⌘K), it opens for editing, to write without dictating.

import { clipboardFiles, documentSize, imageLabel, isImage } from "./agent/attachments";
import { isShortcut } from "./agent/shortcut";
import type { Attachment } from "./api";
import { IconButton } from "./primitives";
import { useEffect, useRef, useState } from "react";
import { cleanArg, isolate, t } from "./i18n";
import { Icon } from "./icons";

/** The draft, its sending and its discarding; nothing without an open draft. */
export function DictationDraft({
  text,
  dictations,
  typing,
  busy,
  ready,
  shortcuts,
  attachments,
  thumbs,
  onAttachFiles,
  onRemoveAttachment,
  onChange,
  onSend,
  onDiscard,
}: {
  /** `null`: no open draft. */
  text: string | null;
  /** Dictations received: each brings the cursor back to the end of the draft. */
  dictations: number;
  /** Requests to type: each opens the field, the cursor at the end. */
  typing: number;
  /** An agent turn is running: sending waits for its end. */
  busy: boolean;
  /** An agent answers: without one nothing is sent (the text stays in the draft). */
  ready: boolean;
  /** The main tab's deck mode: the only case where the host handles a shortcut word with no agent or turn. */
  shortcuts: boolean;
  onChange: (text: string) => void;
  /** The draft's attachments, held by the host. */
  attachments: Attachment[];
  /** Thumbnails of the images attached in this view. */
  thumbs: ReadonlyMap<string, string>;
  /** Files pasted, dropped or picked for the draft, for the host to attach. */
  onAttachFiles: (files: File[]) => void;
  onRemoveAttachment: (id: string) => void;
  onSend: (text: string, attachments: string[]) => void;
  onDiscard: () => void;
}) {
  const area = useRef<HTMLTextAreaElement>(null);
  const note = useRef<HTMLDivElement>(null);
  const picker = useRef<HTMLInputElement>(null);
  const [editing, setEditing] = useState(false);
  // After each dictation the focus comes back to the draft: ⌘↩ right away.
  useEffect(() => {
    if (editing) toEnd(area.current);
    else note.current?.focus();
  }, [dictations]);
  // A request to type opens the field (the next effect puts the cursor in it).
  useEffect(() => {
    if (typing === 0) return;
    if (editing) toEnd(area.current);
    else setEditing(true);
  }, [typing]);
  // Opened for editing: the cursor at the end of the text. Closed: the focus stays on the card.
  const opened = useRef(false);
  useEffect(() => {
    if (editing) toEnd(area.current);
    else if (opened.current) note.current?.focus();
    opened.current = editing;
  }, [editing]);

  if (text === null) return null;
  const said = text.trim();
  // Leaving a draft still empty closes it: there is nothing to keep.
  const leave = () => {
    if (said === "") onDiscard();
    else setEditing(false);
  };
  // A shortcut word ("theme") always goes in deck mode: the host opens it with no agent or turn.
  // Elsewhere it would go to the agent: the same guards as any text.
  const canSend = said !== "" && ((shortcuts && isShortcut(said)) || (!busy && ready));
  const send = () => {
    if (canSend) onSend(said, attachments.map((i) => i.id));
  };
  return (
    <section
      className="draft card prompt"
      aria-label={t("ui-draft")}
      onPaste={(e) => {
        const files = clipboardFiles(e.clipboardData);
        if (files.length === 0) return;
        e.preventDefault();
        onAttachFiles(files);
      }}
      // Files dropped from the system, in the browser. In the Tauri app the webview takes OS file
      // drops (t-819): the picker button and paste are the way in there.
      onDragOver={(e) => {
        if (!e.dataTransfer.types.includes("Files")) return;
        e.preventDefault();
        e.dataTransfer.dropEffect = "copy";
      }}
      onDrop={(e) => {
        const files = [...e.dataTransfer.files];
        if (files.length === 0) return;
        e.preventDefault();
        onAttachFiles(files);
      }}
      onKeyDown={(e) => {
        if (e.key !== "Enter" || !(e.metaKey || e.ctrlKey)) return;
        // ⌘⇧↩ does not present the deck from the draft: only ⌘↩ does something here.
        e.preventDefault();
        e.stopPropagation();
        if (!e.shiftKey) send();
      }}
    >
      {editing ? (
        <textarea
          ref={area}
          className="draft-text"
          value={text}
          rows={3}
          onChange={(e) => onChange(e.target.value)}
          onBlur={leave}
          onKeyDown={(e) => {
            if (e.key !== "Escape") return;
            e.stopPropagation();
            leave();
          }}
        />
      ) : (
        <div
          ref={note}
          className="draft-note"
          role="button"
          tabIndex={0}
          onClick={() => setEditing(true)}
          onKeyDown={(e) => {
            if (e.key !== "Enter" || e.metaKey || e.ctrlKey) return;
            e.preventDefault();
            e.stopPropagation();
            setEditing(true);
          }}
        >
          {text}
        </div>
      )}
      {attachments.length > 0 && (
        <ul className="draft-images" aria-label={t("ui-draft-attachments")}>
          {attachments.map((a) =>
            isImage(a) ? (
              <li key={a.id} className="draft-image">
                {thumbs.get(a.id) && <img src={thumbs.get(a.id)} alt="" draggable={false} />}
                <span>{imageLabel(a.mime, a.width, a.height)}</span>
                <IconButton className="token ghost" label={t("ui-draft-remove-image")} icon={Icon.close} onClick={() => onRemoveAttachment(a.id)} />
              </li>
            ) : (
              // The name is the user's file name: text only, isolated, cut by CSS.
              <li key={a.id} className="draft-document">
                <span className="draft-name" title={cleanArg(a.name)}>{isolate(a.name)}</span>
                <span className="draft-size">{documentSize(a.bytes)}</span>
                <IconButton className="token ghost" label={t("ui-attachment-remove", { name: a.name })} icon={Icon.close} onClick={() => onRemoveAttachment(a.id)} />
              </li>
            ),
          )}
        </ul>
      )}
      <div className="draft-actions">
        <input
          ref={picker}
          type="file"
          multiple
          hidden
          onChange={(e) => {
            const files = [...(e.target.files ?? [])];
            // Emptied so that picking the same file again is a change.
            e.target.value = "";
            if (files.length > 0) onAttachFiles(files);
          }}
        />
        <IconButton className="token ghost draft-pick" label={t("ui-attach-files")} icon={Icon.paperclip} onClick={() => picker.current?.click()} />
        <button type="button" className="token ghost draft-discard" aria-label={t("ui-draft-discard")} title={t("ui-draft-discard")} onClick={onDiscard}>{Icon.trash}</button>
        <button type="button" className="object token primary draft-send" disabled={!canSend} aria-label={t("ui-draft-send")} title={said === "" && attachments.length > 0 ? t("ui-draft-send-needs-text") : t("ui-draft-send-title")} onClick={send}>
          {Icon.send}
        </button>
      </div>
    </section>
  );
}

/** Focuses the field, the cursor at the end of the text. */
function toEnd(el: HTMLTextAreaElement | null) {
  if (!el) return;
  el.focus();
  el.setSelectionRange(el.value.length, el.value.length);
}
