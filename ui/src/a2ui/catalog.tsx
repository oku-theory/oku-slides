// Catalogue A2UI d'Oku Slides (v3) : réglages du dock et surfaces de l'agent. Rendu par
// @a2ui/react ; les messages sont validés contre ces schémas zod par
// @a2ui/web_core.
//
// Noms et propriétés reprennent le catalogue de base v0.9 ; `RichTextField`, `Stepper`, `ListTable`,
// `ChoiceChips`, `ColorField`, `SlideThumb`, `ThemeChoice`, `DiffText`, `Prompt`, `Timeline`,
// `Receipt`, `Disclosure`, `Badge`, `Markdown`, `Notice`, `ChartData`, `TableData` sont propres à Oku Slides.
// Miroir du crate Rust oku-slides-a2ui (même CATALOG_ID).

import { useContext, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { z } from "zod";
import { createComponentImplementation } from "@a2ui/react/v0_9";
import { Catalog, CommonSchemas, type ComponentContext } from "@a2ui/web_core/v0_9";
import type { Paragraph } from "../api";
import { t } from "../i18n";
import { Icon } from "../icons";
import { Button as UiButton, Field, IconButton, Select as UiSelect } from "../primitives";
import { ActionOutcomeContext, sendWithOutcome } from "./actionOutcome";
import { QuickHintsContext } from "./quickHints";
import { ListTable, type ListRow } from "./ListTable";
import { RichTextField } from "./RichTextField";
import { StagePreviewContext } from "./stagePreview";
import { Hex, ToneSchema, toneClass } from "./schemas";
import { svgSrc } from "./svgSrc";
import { ThemeChoice } from "./ThemeChoice";
import { ChartData, TableData } from "./dataComponents";
import { Badge, Disclosure, Markdown, Prompt, Receipt, Timeline } from "./turnComponents";

export const CATALOG_ID = "urn:oku-slides:a2ui:catalog:v4";

const ColumnApi = { name: "Column", schema: z.object({ children: CommonSchemas.ChildList }).strict() };
const CardApi = { name: "Card", schema: z.object({ child: CommonSchemas.ComponentId, tone: ToneSchema.optional() }).strict() };
const TextApi = {
  name: "Text",
  schema: z.object({ text: CommonSchemas.DynamicString, variant: z.enum(["h4", "body", "caption", "note"]).optional() }).strict(),
};
const TextFieldApi = {
  name: "TextField",
  schema: z
    .object({ label: CommonSchemas.DynamicString, value: CommonSchemas.DynamicString, action: CommonSchemas.Action.optional() })
    .strict(),
};
const RichTextFieldApi = {
  name: "RichTextField",
  schema: z
    .object({
      label: CommonSchemas.DynamicString,
      value: CommonSchemas.DynamicString,
      source: CommonSchemas.DynamicValue,
      palette: z.array(Hex).readonly(),
      action: CommonSchemas.Action.optional(),
    })
    .strict(),
};
const StepperApi = {
  name: "Stepper",
  schema: z
    .object({
      label: CommonSchemas.DynamicString,
      value: CommonSchemas.DynamicNumber,
      min: z.number().int().optional(),
      max: z.number().int().optional(),
      action: CommonSchemas.Action.optional(),
    })
    .strict(),
};
const Option = z.object({ id: z.string(), label: z.string(), hint: z.string().optional() }).strict();
/** Des options aux identifiants uniques : une valeur choisie doit en désigner une seule. */
const Options = z.array(Option).refine((os) => new Set(os.map((o) => o.id)).size === os.length, {
  message: "option ids must be unique",
});
const RowApi = { name: "Row", schema: z.object({ children: CommonSchemas.ChildList }).strict() };
const ButtonApi = {
  name: "Button",
  schema: z
    .object({
      label: CommonSchemas.DynamicString,
      primary: z.boolean().optional(),
      danger: z.boolean().optional(),
      icon: z.enum(["close", "up", "down", "remove"]).optional(),
      action: CommonSchemas.Action,
    })
    .strict(),
};
const ChoiceChipsApi = {
  name: "ChoiceChips",
  schema: z
    .object({ options: Options, value: CommonSchemas.DynamicString, action: CommonSchemas.Action.optional() })
    .strict(),
};
const SliderApi = {
  name: "Slider",
  schema: z
    .object({
      label: CommonSchemas.DynamicString,
      value: CommonSchemas.DynamicNumber,
      min: z.number(),
      max: z.number(),
      step: z.number().optional(),
      action: CommonSchemas.Action.optional(),
    })
    .strict(),
};
/**
 * `SliderApi.schema` plus l'ordre des bornes, vérifié par `buildSurface` : @a2ui/web_core lit la
 * forme d'un `ZodObject` pour lier les valeurs, ce que ne donne pas un `ZodEffects`.
 */
export const CheckedSlider = SliderApi.schema.refine((s) => s.min < s.max, {
  message: "min must be < max",
  path: ["min"],
});
const ColorFieldApi = {
  name: "ColorField",
  schema: z
    .object({
      label: CommonSchemas.DynamicString,
      value: CommonSchemas.DynamicString,
      // `readonly` : un `string[]` serait pris pour une liste d'enfants (`ChildList`) par @a2ui/react.
      palette: z.array(Hex).readonly(),
      action: CommonSchemas.Action.optional(),
    })
    .strict(),
};
const SelectApi = {
  name: "Select",
  schema: z
    .object({
      label: CommonSchemas.DynamicString,
      value: CommonSchemas.DynamicString,
      options: Options,
      action: CommonSchemas.Action.optional(),
    })
    .strict(),
};
const ListColumnBase = {
  key: z.string(),
  label: z.string(),
  kind: z.enum(["text", "rich", "number", "flag", "select", "asset", "list"]),
  options: Options,
};
/** Colonne d'une liste imbriquée : jamais elle-même une liste (un seul niveau, miroir de `SubList`). */
const SubColumnSchema = z.object({ ...ListColumnBase, kind: ListColumnBase.kind.exclude(["list"]) }).strict();
const ListColumnSchema = z
  .object({
    ...ListColumnBase,
    list: z
      .object({
        columns: z.array(SubColumnSchema),
        min: z.number().int(),
        max: z.number().int().nullable().optional(),
        item_label: z.string().nullable().optional(),
      })
      .strict()
      // Miroir de `SubList::new` : `max` au moins `min`.
      .refine((l) => l.max == null || l.min <= l.max, { message: "max < min" })
      .nullable()
      .optional(),
  })
  .strict();
const ListTableApi = {
  name: "ListTable",
  schema: z
    .object({
      label: CommonSchemas.DynamicString,
      rows: CommonSchemas.DynamicValue,
      change: CommonSchemas.DynamicValue,
      markdown: CommonSchemas.DynamicValue,
      columns: z.array(ListColumnSchema),
      min: z.number().int(),
      max: z.number().int().nullable().optional(),
      item_label: z.string().nullable().optional(),
      palette: z.array(Hex).readonly(),
      action: CommonSchemas.Action.optional(),
    })
    .strict(),
};
const SlideThumbApi = {
  name: "SlideThumb",
  schema: z.object({ svg: z.string(), label: CommonSchemas.DynamicString, action: CommonSchemas.Action.optional() }).strict(),
};
const NoticeApi = {
  name: "Notice",
  schema: z
    .object({
      text: CommonSchemas.DynamicString,
      children: CommonSchemas.ChildList,
      close: CommonSchemas.Action,
      close_label: CommonSchemas.DynamicString,
    })
    .strict(),
};
const DiffTextApi = {
  name: "DiffText",
  schema: z
    .object({ segments: z.array(z.object({ kind: z.enum(["keep", "del", "ins"]), text: z.string() }).strict()) })
    .strict(),
};

type ChildRef = string | { id: string; basePath: string };

/**
 * L'id de composant du `i`-ième enfant, lu dans la `ChildList` brute du modèle :
 * depuis @a2ui 0.13, `props.children` ne porte que des jetons de nœud opaques.
 * Une liste fixe s'aligne indice pour indice ; un gabarit a un seul `componentId`.
 */
function childComponentId(context: ComponentContext, i: number): string {
  const raw: unknown = context.componentModel.properties.children;
  const id = Array.isArray(raw)
    ? raw[i]
    : typeof raw === "object" && raw !== null && "componentId" in raw
      ? raw.componentId
      : undefined;
  // Le schéma ChildList a validé cette forme : un écart est un bogue, pas une donnée.
  if (typeof id !== "string") throw new Error(`ChildList inattendue pour ${context.componentModel.id}`);
  return id;
}

/**
 * Les enfants d'une colonne ou d'une rangée, chacun dans son emplacement.
 * L'emplacement porte l'id du composant (`data-component-id`) : une prise
 * stable pour les tests de bout en bout (une ligne de jeton : `color.accent:row`).
 */
function children(
  refs: unknown,
  buildChild: (id: string, basePath?: string) => ReactNode,
  context: ComponentContext,
  slot?: string,
) {
  if (!Array.isArray(refs)) return null;
  return (refs as ChildRef[]).map((ref, i) =>
    typeof ref === "string"
      ? <div key={ref} className={slot} data-component-id={childComponentId(context, i)}>{buildChild(ref)}</div>
      : <div key={`${ref.id}-${ref.basePath}`} className={slot} data-component-id={childComponentId(context, i)}>
          {buildChild(ref.id, ref.basePath)}
        </div>,
  );
}

/** Enfants empilés. */
const Column = createComponentImplementation(ColumnApi, ({ props, buildChild, context }) => (
  <div className="a2ui-column">{children(props.children, buildChild, context, "a2ui-slot")}</div>
));

/** Une carte autour d'un seul enfant. */
const Card = createComponentImplementation(CardApi, ({ props, buildChild }) => (
  <div className={`a2ui-card${toneClass(props.tone)}`}>{buildChild(props.child)}</div>
));

const TEXT_CLASS = { h4: "a2ui-title", body: undefined, caption: "a2ui-hint", note: "a2ui-note" } as const;

/** Titre (h4), texte courant, légende ou note. */
const Text = createComponentImplementation(TextApi, ({ props }) => (
  <span className={TEXT_CLASS[props.variant ?? "body"]}>{props.text}</span>
));

/** Champ d'une carte : la valeur et l'action partent à la sortie ou sur Entrée. */
const TextField = createComponentImplementation(TextFieldApi, ({ props }) => {
  const value = props.value ?? "";
  const [draft, setDraft] = useState(value);
  useEffect(() => setDraft(value), [value]);
  const commit = () => {
    if (draft === value) return;
    props.setValue(draft);
    props.action?.();
  };
  return (
    <Field variant="note" label={props.label}>
      <input
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") e.currentTarget.blur();
          else if (e.key === "Escape") { setDraft(value); e.currentTarget.blur(); }
          // Les raccourcis de l'app ne doivent pas voir la saisie.
          e.stopPropagation();
        }}
      />
    </Field>
  );
});

/** Texte riche multiligne : markdown et paragraphes écrits, puis l'action. */
const RichText = createComponentImplementation(RichTextFieldApi, ({ props }) => (
  <RichTextField
    label={props.label}
    value={props.value ?? ""}
    source={Array.isArray(props.source) ? (props.source as Paragraph[]) : []}
    palette={[...props.palette]}
    onCommit={(markdown, paragraphs) => {
      props.setValue(markdown);
      props.setSource(paragraphs);
      props.action?.();
    }}
  />
));

/** Petit compteur − valeur + ; l'action part à chaque pas. */
const Stepper = createComponentImplementation(StepperApi, ({ props }) => {
  const { label, value, min = 1, max = 24 } = props;
  const step = (next: number) => {
    props.setValue(next);
    props.action?.();
  };
  return (
    <div className="stepper">
      <span className="a2ui-hint">{label}</span>
      <button aria-label={t("ui-stepper-less", { label })} disabled={value <= min} onClick={() => step(value - 1)}>−</button>
      <span className="stepper-value">{value}</span>
      <button aria-label={t("ui-stepper-more", { label })} disabled={value >= max} onClick={() => step(value + 1)}>+</button>
    </div>
  );
});

/** Enfants côte à côte. */
const Row = createComponentImplementation(RowApi, ({ props, buildChild, context }) => (
  <div className="a2ui-row">{children(props.children, buildChild, context)}</div>
));

/** Une valeur qui part à l'agent au plus une fois (pas de renvoi de la valeur déjà envoyée). */
function useCommit<T>(value: T | undefined, send: (v: T) => void) {
  const sent = useRef(value);
  useEffect(() => {
    sent.current = value;
  }, [value]);
  return (v: T) => {
    if (v === sent.current) return;
    sent.current = v;
    send(v);
  };
}

/** L'évènement natif `change` d'un champ : la valeur validée (curseur lâché, sélecteur fermé). */
function useNativeChange(onChange: (value: string) => void) {
  const ref = useRef<HTMLInputElement>(null);
  const handler = useRef(onChange);
  handler.current = onChange;
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const listener = () => handler.current(el.value);
    el.addEventListener("change", listener);
    return () => el.removeEventListener("change", listener);
  }, []);
  return ref;
}

/** Nombre de décimales d'un pas (0.25 → 2), pour afficher la valeur sans bruit flottant. */
const decimals = (step: number) => (String(step).split(".")[1] ?? "").length;

/** `#abc` → `#aabbcc`, `#rrggbbaa` → `#rrggbb` : le sélecteur natif ne lit que six chiffres. */
export function sixDigits(color: string | undefined): string {
  const hex = /^#([0-9a-f]+)$/i.exec(color ?? "")?.[1] ?? "";
  if (hex.length === 3 || hex.length === 4) return `#${[...hex.slice(0, 3)].map((c) => c + c).join("")}`.toLowerCase();
  if (hex.length === 6 || hex.length === 8) return `#${hex.slice(0, 6)}`.toLowerCase();
  return "#000000";
}

/**
 * Un bouton ; `primary` marque l'action principale (Entrée dans une fiche de l'agent) ;
 * `danger`, une action qui engage (non annulable, permission permanente) ; `icon`, un
 * pictogramme seul dont le libellé devient nom accessible et infobulle.
 */
const Button = createComponentImplementation(ButtonApi, ({ props }) =>
  props.icon ? (
    <IconButton
      label={props.label}
      icon={Icon[props.icon]}
      className={["object token a2ui-icon-button", props.primary && "primary", props.danger && "danger"].filter(Boolean).join(" ")}
      data-primary={props.primary ? "" : undefined}
      onClick={() => props.action()}
    />
  ) : (
    <UiButton
      raised
      primary={props.primary}
      danger={props.danger}
      data-primary={props.primary ? "" : undefined}
      onClick={() => props.action()}
    >
      {props.label}
    </UiButton>
  ),
);

/** Touches rapides montrées : 1 à 4 (Task 20 les lit). */
const QUICK_KEYS = 4;

/** Des pastilles, une seule choisie ; choisir écrit l'identifiant puis envoie l'action. Chiffres 1–4 : voir `QuickHintsContext`. */
const ChoiceChips = createComponentImplementation(ChoiceChipsApi, ({ props }) => {
  const hints = useContext(QuickHintsContext);
  return (
    <div className="chips" role="radiogroup">
      {props.options.map((o, i) => (
        <button
          key={`${i}-${o.id}`}
          type="button"
          role="radio"
          aria-checked={props.value === o.id}
          className={`chip${props.value === o.id ? " on" : ""}`}
          title={o.hint}
          onClick={() => {
            props.setValue(o.id);
            props.action?.();
          }}
        >
          {hints && i < QUICK_KEYS && <span className="chip-key" aria-hidden="true">{i + 1}</span>} {o.label}
        </button>
      ))}
    </div>
  );
});

/** Un curseur ; l'action part quand la valeur est validée (lâchée, ou à la sortie du champ). */
const Slider = createComponentImplementation(SliderApi, ({ props }) => {
  const [draft, setDraft] = useState(props.value ?? props.min);
  useEffect(() => setDraft(props.value ?? props.min), [props.value, props.min]);
  const commit = useCommit(props.value, (v: number) => {
    props.setValue(v);
    props.action?.();
  });
  const ref = useNativeChange((v) => commit(Number(v)));
  const step = props.step ?? 1;
  return (
    <Field variant="note" label={<>{props.label} · {draft.toFixed(decimals(step))}</>}>
      <input
        ref={ref}
        type="range"
        min={props.min}
        max={props.max}
        step={step}
        value={draft}
        onChange={(e) => setDraft(Number(e.target.value))}
        onBlur={() => commit(draft)}
      />
    </Field>
  );
});

/**
 * Une couleur : pastilles de la palette, ou sélecteur libre (validé à sa fermeture). La valeur
 * peut être le nom d'une couleur du thème (`accent`) : montré tel quel ; choisir écrit un hex.
 */
const ColorField = createComponentImplementation(ColorFieldApi, ({ props }) => {
  const named = !!props.value && !props.value.startsWith("#");
  const [draft, setDraft] = useState(sixDigits(props.value));
  useEffect(() => setDraft(sixDigits(props.value)), [props.value]);
  const pick = useCommit(props.value, (c: string) => {
    props.setValue(c);
    props.action?.();
  });
  const ref = useNativeChange(pick);
  return (
    <Field variant="note" label={props.label} as="div">
      <div className="swatches">
        {props.palette.map((c, i) => (
          <button
            key={`${i}-${c}`}
            type="button"
            className={`swatch${c === props.value ? " on" : ""}`}
            style={{ background: c }}
            aria-label={c}
            aria-pressed={c === props.value}
            onClick={() => pick(c)}
          />
        ))}
        <input
          ref={ref}
          type="color"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          aria-label={t("ui-agent-color-other")}
        />
        {named && <span className="a2ui-hint color-name">{props.value}</span>}
      </div>
    </Field>
  );
});

/** Une liste déroulante ; une valeur absente des options montre une option vide. */
const Select = createComponentImplementation(SelectApi, ({ props }) => {
  const known = props.options.some((o) => o.id === props.value);
  return (
    <Field variant="note" label={props.label}>
      <UiSelect
        value={known ? props.value : ""}
        options={[...(known ? [] : [{ value: "", label: "", disabled: true }]), ...props.options.map((o) => ({ value: o.id, label: o.label }))]}
        onChange={(v) => {
          props.setValue(v);
          props.action?.();
        }}
      />
    </Field>
  );
});

/** Les items d'une liste : objets à `id` chaîne ; le reste est ignoré. */
const listRows = (v: unknown): ListRow[] =>
  Array.isArray(v) ? v.filter((r): r is ListRow => typeof r === "object" && r !== null && typeof r.id === "string") : [];

/** Markdown des cellules riches, par id puis colonne (un objet, malgré le type de `DynamicValue`). */
const listMarkdown = (v: unknown): Record<string, Record<string, string>> =>
  typeof v === "object" && v !== null && !Array.isArray(v) ? (v as Record<string, Record<string, string>>) : {};

/** Le tableau d'une liste : le changement est écrit, puis l'action part ; son issue revient au tableau. */
const ListTableField = createComponentImplementation(ListTableApi, ({ props }) => {
  const outcome = useContext(ActionOutcomeContext);
  return (
    <ListTable
      label={props.label}
      rows={listRows(props.rows)}
      columns={props.columns}
      min={props.min}
      max={props.max ?? null}
      itemLabel={props.item_label ?? null}
      palette={[...props.palette]}
      markdown={listMarkdown(props.markdown)}
      onChange={(c) => {
        // `DynamicValue` est typé sans objet ; le modèle de données en garde un tel quel.
        (props.setChange as (v: unknown) => void)(c);
        return sendWithOutcome(outcome, () => props.action?.());
      }}
    />
  );
});

/**
 * Miniature d'une slide rendue par la session ; image (le SVG n'est pas exécuté).
 * Survolée ou au focus, la scène montre son rendu ; quittée ou démontée, la slide.
 */
const SlideThumb = createComponentImplementation(SlideThumbApi, ({ props }) => {
  const src = useMemo(() => svgSrc(props.svg), [props.svg]);
  const store = useContext(StagePreviewContext);
  // Survol et focus se suivent à part : quitter l'un garde l'aperçu tant que l'autre dure.
  const [hovered, setHovered] = useState(false);
  const [focused, setFocused] = useState(false);
  const on = hovered || focused;
  // Posé tant que la miniature est survolée ou focalisée ; retiré quand elle ne l'est plus,
  // change de rendu ou est démontée (tiroir fermé, variante choisie).
  useEffect(() => {
    if (!store || !on) return;
    const preview = { url: src };
    store.show(preview);
    return () => store.hide(preview);
  }, [store, on, src]);
  return (
    <button
      type="button"
      className="slide-thumb"
      onClick={() => props.action?.()}
      onPointerEnter={() => setHovered(true)}
      onPointerLeave={() => setHovered(false)}
      onFocus={() => setFocused(true)}
      onBlur={() => setFocused(false)}
    >
      <img src={src} alt={props.label} />
      <span className="caption">{props.label}</span>
    </button>
  );
});

/** Un texte et ses changements, mot à mot. */
const DiffText = createComponentImplementation(DiffTextApi, ({ props }) => (
  <p className="diff-text">
    {props.segments.map((s, i) =>
      s.kind === "del" ? <del key={i}>{s.text}</del> : s.kind === "ins" ? <ins key={i}>{s.text}</ins> : <span key={i}>{s.text}</span>,
    )}
  </p>
));

/** Un bandeau annoncé : le texte, les boutons enfants, puis la croix qui le ferme. */
const Notice = createComponentImplementation(NoticeApi, ({ props, buildChild, context }) => (
  <div className="notice-bar" role="status">
    <span className="notice-bar-text">{props.text}</span>
    <div className="notice-bar-actions">{children(props.children, buildChild, context)}</div>
    <IconButton
      label={props.close_label}
      icon={Icon.close}
      className="token ghost notice-bar-close"
      onClick={() => props.close()}
    />
  </div>
));

/** Le catalogue des surfaces d'Oku Slides : réglages du dock et agent. */
export const catalog = new Catalog(CATALOG_ID, "0.9", [
  Column, Row, Card, Text, TextField, RichText, Stepper, Button, ChoiceChips, Slider, ColorField, Select, ListTableField, SlideThumb, ThemeChoice, DiffText,
  Prompt, Timeline, Receipt, Disclosure, Badge, Markdown, Notice, ChartData, TableData,
]);
