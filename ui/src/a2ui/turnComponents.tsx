// Composants du fil de l'agent (catalogue v3) : consigne, timeline, reçu,
// détail repliable, badge, Markdown. Hors de catalog.tsx : un concept par fichier.

import { useContext } from "react";
import { z } from "zod";
import { createComponentImplementation } from "@a2ui/react/v0_9";
import { CommonSchemas, type ComponentContext } from "@a2ui/web_core/v0_9";
import { Markdown as MarkdownView } from "../agent/Markdown";
import { t } from "../i18n";
import { useRememberedOpen } from "./openMemory";
import { Hex, ToneSchema, toneClass } from "./schemas";
import { SlideRefSchema, SlideThumbsContext, resolveSlideThumb } from "./slideThumbs";
import { Toggle } from "./Toggle";

const Status = z.enum(["pending", "in_progress", "completed", "failed"]);
const Call = z.object({
  label: z.string(),
  status: Status,
  input: z.string().optional(),
  output: z.string().optional(),
}).strict();
const Step = z.object({
  label: z.string(),
  status: Status,
  count: z.number().int().optional(),
  /** Ce que le libellé ne montre pas d'une note : sa suite, ou son texte entier s'il est coupé. */
  detail: z.string().optional(),
  /** Les appels d'une étape d'outils. */
  calls: z.array(Call).optional(),
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
  // `open` : déplié au premier rendu (une section des jetons, toutes pendant un filtre).
  schema: z.object({ summary: CommonSchemas.DynamicString, child: CommonSchemas.ComponentId, open: z.boolean().optional() }).strict(),
};
const BadgeApi = { name: "Badge", schema: z.object({ text: CommonSchemas.DynamicString, tone: ToneSchema.optional() }).strict() };
const MarkdownApi = { name: "Markdown", schema: z.object({ text: CommonSchemas.DynamicString }).strict() };

/** Marque d'une étape selon son statut. */
const MARK: Record<z.infer<typeof Status>, string> = { pending: "○", in_progress: "◌", completed: "✓", failed: "✕" };

/** Longueur lue des arguments pour leur résumé d'une ligne (l'ellipse CSS coupe avant). */
const ARGS_PREVIEW = 200;

/** Un appel d'outil : nom et arguments sur une ligne ; déplié, arguments et résultat entiers. */
function CallRow({ call, context, part }: { call: z.infer<typeof Call>; context: ComponentContext; part: string }) {
  const [open, setOpen] = useRememberedOpen(context, false, part);
  // Un texte vide n'a rien à montrer : ni résumé, ni bloc.
  const input = call.input || undefined;
  const output = call.output || undefined;
  const head = (
    <>
      <span aria-hidden="true">{MARK[call.status]}</span>
      <span className="a2ui-call-label">{call.label}</span>
      {input && <span className="a2ui-call-args">{input.slice(0, ARGS_PREVIEW).replace(/\s+/g, " ")}</span>}
    </>
  );
  const io = input !== undefined || output !== undefined;
  return (
    // `status` vient de l'énumération de `Call` : une valeur sûre.
    <li className={`a2ui-call ${call.status}`}>
      {io ? <Toggle open={open} onToggle={() => setOpen(!open)} className="a2ui-call-toggle">{head}</Toggle> : <div className="a2ui-call-row">{head}</div>}
      {io && open && (
        <dl className="a2ui-call-io">
          {input !== undefined && (<><dt>{t("ui-agent-tool-input")}</dt><dd><pre>{input}</pre></dd></>)}
          {output !== undefined && (<><dt>{t("ui-agent-tool-output")}</dt><dd><pre>{output}</pre></dd></>)}
        </dl>
      )}
    </li>
  );
}

/** Une étape de la timeline ; avec un détail (texte entier, appels), elle se déplie. */
function StepRow({ step, context, part }: { step: z.infer<typeof Step>; context: ComponentContext; part: string }) {
  const [open, setOpen] = useRememberedOpen(context, false, part);
  const head = (
    <>
      <span aria-hidden="true">{MARK[step.status]}</span>
      <span className="a2ui-step-label">{step.label}</span>
      {step.count !== undefined && <span className="a2ui-badge">{step.count}</span>}
    </>
  );
  const calls = step.calls ?? [];
  const more = step.detail !== undefined || calls.length > 0;
  // Rang de chaque appel dans l'étape, plus anciens sortis de la liste compris : stable quand le
  // plus ancien sort (un long tour), sa ligne dépliée le reste.
  const first = (step.count ?? calls.length) - calls.length;
  return (
    // `status` vient de l'énumération de `Step` : une valeur sûre.
    <li className={`a2ui-step ${step.status}`}>
      {more ? <Toggle open={open} onToggle={() => setOpen(!open)} className="a2ui-step-toggle">{head}</Toggle> : <div className="a2ui-step-row">{head}</div>}
      {open && step.detail !== undefined && <div className="a2ui-step-detail md"><MarkdownView text={step.detail} /></div>}
      {open && calls.length > 0 && (
        <ol className="a2ui-calls">
          {calls.map((c, i) => <CallRow key={first + i} call={c} context={context} part={`${part}.${first + i}`} />)}
        </ol>
      )}
    </li>
  );
}

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
  // Clé d'une étape d'outils ou d'une note détaillée : son rang parmi elles. Ni l'une ni l'autre
  // ne change de nature (une étape d'outils garde son compte même vidée par le plafond), et le
  // plan, en tête de liste, n'en est jamais : l'état déplié ne passe pas à l'étape voisine.
  let ranked = 0;
  const parts = steps.map((s, i) => (s.count !== undefined || s.calls !== undefined || s.detail !== undefined ? `d${ranked++}` : `p${i}`));
  return (
    <div className="a2ui-timeline">
      {props.collapsed && (
        <Toggle open={open} onToggle={() => setOpen(!open)} className="a2ui-timeline-summary">{props.summary}</Toggle>
      )}
      {shown && (
        <ol className="a2ui-steps">
          {steps.map((s, i) => <StepRow key={parts[i]} step={s} context={context} part={parts[i]} />)}
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

/**
 * Un enfant replié derrière une ligne (déplié d'abord si `open`) ; ouvert ou fermé
 * à la main, il le reste si la surface est reconstruite.
 */
export const Disclosure = createComponentImplementation(DisclosureApi, ({ props, buildChild, context }) => {
  const [open, setOpen] = useRememberedOpen(context, props.open ?? false);
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
