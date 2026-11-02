/**
 * The "Guides for agents" of a theme tab, and one guide per tab in a pack's Commun (`GuidePanel`):
 * the two guides the user writes (how to edit, how to use), with a live count against the cap and
 * the origin of the edited version.
 *
 * A field saves on blur, and only when its text changed. A received version not yet approved shows
 * its text with an "Approve this guide" button; editing it makes a local version, which needs none.
 * The button shows only when the session says the text shown is the text approved (`approvable`),
 * and is off while a field holds unsaved text: a click would approve the version being replaced.
 * In a pack's Commun, each guide is a `GuidePanel` and the caller places one `Approval` under them,
 * also off until both guides were shown.
 * A theme draft mixing written and kept guides (`kept_awaiting`) gets a line saying how to approve.
 * Guide text is untrusted: it is only ever a text node, never Markdown or HTML.
 */
import { useCallback, useEffect, useState } from "react";
import type { GuideKind, GuidesView, Snapshot, TabApi } from "./api";
import { t } from "./i18n";
import { Button } from "./primitives";

/** Applies a request that answers a snapshot; `null` when nothing was done. */
export type Run = (p: Promise<Snapshot>) => Promise<Snapshot | null>;

/** The badge text of each origin (literal ids, so unused messages are caught). */
const ORIGIN = {
  builtin: "ui-guides-origin-builtin",
  local: "ui-guides-origin-local",
  received: "ui-guides-origin-received",
} as const;

/** Characters of `text`, as the session counts them (code points, not UTF-16 units). */
const length = (text: string) => [...text].length;

/** Reports whether a guide's field holds unsaved text. */
export type OnGuideDirty = (which: GuideKind, dirty: boolean) => void;

/** One guide's field: labelled, capped, saved on blur when changed; `onDirty` tells when it differs. */
function GuideField({ which, saved, max, onSave, onDirty }: {
  which: GuideKind;
  saved: string;
  max: number;
  onSave: (text: string) => Promise<boolean>;
  onDirty: OnGuideDirty;
}) {
  const [draft, setDraft] = useState(saved);
  // The session's text wins whenever it changes (an undo, the agent's own write).
  useEffect(() => setDraft(saved), [saved]);
  useEffect(() => onDirty(which, draft !== saved), [which, draft, saved, onDirty]);
  const commit = () => {
    if (draft === saved) return;
    void onSave(draft).then((done) => {
      // Refused (too long, read-only): the field takes the session's text back.
      if (!done) setDraft(saved);
    });
  };
  return (
    <label className="note-field guide-field">
      <span className="caption">
        {t(which === "edit" ? "ui-guides-edit" : "ui-guides-use")}
        <span className="guide-count">{t("ui-guides-count", { count: length(draft), max })}</span>
      </span>
      <textarea
        value={draft}
        rows={4}
        maxLength={max}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        // The app's shortcuts do not see the typing.
        onKeyDown={(e) => e.stopPropagation()}
      />
    </label>
  );
}

/** The version of `view` is received and not approved yet: its guides do not reach the agent. */
export const awaitingApproval = (view: GuidesView) => view.origin === "received" && !view.approved;

/** The origin badge of the edited version. */
function OriginBadge({ view }: { view: GuidesView }) {
  return <span className="guides-badge" data-origin={view.origin}>{t(ORIGIN[view.origin])}</span>;
}

/** While awaiting, the note and, when the session allows it, the button approving the version. */
export function Approval({ api, view, run, blocked }: { api: TabApi; view: GuidesView; run: Run; blocked: boolean }) {
  if (!awaitingApproval(view)) return null;
  return (
    <div className="guides-approval">
      <span className="caption">{t("ui-guides-awaiting")}</span>
      {/* A theme draft mixing written and kept guides awaits without a button: say how to approve. */}
      {view.kept_awaiting && <span className="caption">{t("ui-guides-awaiting-mixed")}</span>}
      {view.approvable && (
        <Button
          primary
          disabled={blocked}
          onClick={() => void run(api.approveGuides([{ id: view.id, version: view.version }]))}
        >
          {t("ui-guides-approve")}
        </Button>
      )}
    </div>
  );
}

/** Saves one guide of the edited version; `true` when the session took it. */
const saver = (api: TabApi, run: Run) => (which: GuideKind) => async (text: string) =>
  (await run(api.setGuide(which, text))) !== null;

/** The guides of the edited version of `view`, written through `api.setGuide`. */
export default function GuidesSection({ api, view, run }: { api: TabApi; view: GuidesView; run: Run }) {
  const [dirty, setDirty] = useState<Record<GuideKind, boolean>>({ edit: false, use: false });
  const onDirty = useCallback(
    (which: GuideKind, d: boolean) => setDirty((prev) => (prev[which] === d ? prev : { ...prev, [which]: d })),
    [],
  );
  const save = saver(api, run);
  return (
    <details className="guides-section" open={awaitingApproval(view) || undefined}>
      <summary className="guides-title">
        {t("ui-guides-title")}
        <OriginBadge view={view} />
      </summary>
      <GuideField which="edit" saved={view.guides.edit ?? ""} max={view.max_chars} onSave={save("edit")} onDirty={onDirty} />
      <GuideField which="use" saved={view.guides.use ?? ""} max={view.max_chars} onSave={save("use")} onDirty={onDirty} />
      <Approval api={api} view={view} run={run} blocked={dirty.edit || dirty.use} />
    </details>
  );
}

/** One guide of `view` (a tab of a pack's Commun): its title and origin, then its field. */
export function GuidePanel({ api, view, run, which, onDirty }: {
  api: TabApi;
  view: GuidesView;
  run: Run;
  which: GuideKind;
  onDirty: OnGuideDirty;
}) {
  return (
    <div className="guide-panel">
      <div className="guides-title">
        {t("ui-guides-title")}
        <OriginBadge view={view} />
      </div>
      <GuideField which={which} saved={view.guides[which] ?? ""} max={view.max_chars} onSave={saver(api, run)(which)} onDirty={onDirty} />
    </div>
  );
}
