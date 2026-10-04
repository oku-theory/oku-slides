// Composants du fil de l'agent (catalogue v3) : consigne, timeline, reçu,
// détail repliable, badge, Markdown. Hors de catalog.tsx : un concept par fichier.

import { useContext } from "react";
import { z } from "zod";
import { createComponentImplementation } from "@a2ui/react/v0_9";
import { CommonSchemas } from "@a2ui/web_core/v0_9";
import { Markdown as MarkdownView } from "../agent/Markdown";
import { t } from "../i18n";
import { useRememberedOpen } from "./openMemory";
import { Hex, ToneSchema, toneClass } from "./schemas";
import { SlideRefSchema, SlideThumbsContext, resolveSlideThumb } from "./slideThumbs";
import { Toggle } from "./Toggle";

const Step = z.object({
  label: z.string(),
  status: z.enum(["pending", "in_progress", "completed", "failed"]),
  count: z.number().int().optional(),
}).strict();

const PromptApi = { name: "Prompt", schema: z.object({ text: CommonSchemas.DynamicString }).strict() };
const TimelineApi = {
  name: "Timeline",
  schema: z.object({
    steps: z.union([z.array(Step), z.object({ path: z.string() }).strict()]),
    collapsed: CommonSchemas.DynamicBoolean,
    summary: CommonSchemas.DynamicString,
  }).strict(),
};
const ReceiptApi = {
  name: "Receipt",
  schema: z.object({
    title: CommonSchemas.DynamicString,
    facts: z.array(z.string()).readonly(),
    slide: SlideRefSchema.optional(),
    swatch: Hex.optional(),
  }).strict(),
};
const DisclosureApi = {
  name: "Disclosure",
  schema: z.object({ summary: CommonSchemas.DynamicString, child: CommonSchemas.ComponentId }).strict(),
};
const BadgeApi = { name: "Badge", schema: z.object({ text: CommonSchemas.DynamicString, tone: ToneSchema.optional() }).strict() };
const MarkdownApi = { name: "Markdown", schema: z.object({ text: CommonSchemas.DynamicString }).strict() };

/** Marque d'une étape selon son statut. */
const MARK: Record<z.infer<typeof Step>["status"], string> = { pending: "○", in_progress: "◌", completed: "✓", failed: "✕" };

/** La consigne de l'utilisateur. */
export const Prompt = createComponentImplementation(PromptApi, ({ props }) => (
  <p className="a2ui-prompt">{props.text}</p>
));

/** La progression d'un tour ; repliée, une ligne qui se déplie au clic. */
export const Timeline = createComponentImplementation(TimelineApi, ({ props, context }) => {
  const [open, setOpen] = useRememberedOpen(context);
  // Une valeur liée par chemin n'est pas validée par zod : on ne garde que des étapes bien formées.
  const steps = (Array.isArray(props.steps) ? props.steps : []).filter((x): x is z.infer<typeof Step> => Step.safeParse(x).success);
  const shown = !props.collapsed || open;
  return (
    <div className="a2ui-timeline">
      {props.collapsed && (
        <Toggle open={open} onToggle={() => setOpen(!open)} className="a2ui-timeline-summary">{props.summary}</Toggle>
      )}
      {shown && (
        <ol className="a2ui-steps">
          {steps.map((s, i) => (
            <li key={i} className={`a2ui-step ${s.status}`} /* `status` vient de l'énumération de `Step` : une valeur sûre */>
              <span aria-hidden="true">{MARK[s.status]}</span>
              <span className="a2ui-step-label">{s.label}</span>
              {s.count !== undefined && <span className="a2ui-badge">{s.count}</span>}
            </li>
          ))}
        </ol>
      )}
    </div>
  );
});

/** La conclusion d'un tour ; la miniature de sa slide est résolue par empreinte (`slideThumbs`). */
export const Receipt = createComponentImplementation(ReceiptApi, ({ props }) => {
  const thumbs = useContext(SlideThumbsContext);
  const thumb = props.slide ? resolveSlideThumb(thumbs, props.slide) : null;
  const changed = thumb?.changed ? t("ui-history-changed-since") : undefined;
  return (
    <div className="a2ui-card tone-accent a2ui-receipt">
      {thumb && <img className={`a2ui-receipt-thumb${thumb.changed ? " changed" : ""}`} alt={changed ?? ""} title={changed} src={thumb.url} />}
      {!thumb && props.swatch && <span className="a2ui-swatch" style={{ background: props.swatch }} aria-hidden="true" />}
      <div className="a2ui-receipt-text">
        <span className="a2ui-title">{props.title}</span>
        {props.facts.length > 0 && <span className="a2ui-hint">{props.facts.join(" · ")}</span>}
      </div>
    </div>
  );
});

/** Un enfant replié derrière une ligne ; déplié, il le reste si la surface est reconstruite. */
export const Disclosure = createComponentImplementation(DisclosureApi, ({ props, buildChild, context }) => {
  const [open, setOpen] = useRememberedOpen(context);
  return (
    <div className="a2ui-disclosure">
      <Toggle open={open} onToggle={() => setOpen(!open)} className="a2ui-hint">{props.summary}</Toggle>
      {open && buildChild(props.child)}
    </div>
  );
});

/** Un compteur ou un statut. */
export const Badge = createComponentImplementation(BadgeApi, ({ props }) => (
  <span className={`a2ui-badge${toneClass(props.tone)}`}>{props.text}</span>
));

/** Du Markdown, sans HTML brut. */
export const Markdown = createComponentImplementation(MarkdownApi, ({ props }) => (
  <div className="a2ui-md md"><MarkdownView text={props.text ?? ""} /></div>
));
