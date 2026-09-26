// Catalogue A2UI d'Écho : les composants des surfaces que décrit la session,
// au style Papier (post-it). Rendu par @a2ui/react ; les messages sont
// validés contre ces schémas zod par @a2ui/web_core.
//
// Noms et propriétés reprennent le catalogue de base v0.9 ; `Stepper` est
// propre à Écho. Miroir du crate Rust slider-a2ui (même CATALOG_ID).

import { useEffect, useState } from "react";
import { z } from "zod";
import { createComponentImplementation } from "@a2ui/react/v0_9";
import { Catalog, CommonSchemas } from "@a2ui/web_core/v0_9";
import { t } from "../i18n";

export const CATALOG_ID = "urn:echo:a2ui:catalog:panel:v1";

const ColumnApi = { name: "Column", schema: z.object({ children: CommonSchemas.ChildList }).strict() };
const CardApi = { name: "Card", schema: z.object({ child: CommonSchemas.ComponentId }).strict() };
const TextApi = {
  name: "Text",
  schema: z.object({ text: CommonSchemas.DynamicString, variant: z.enum(["h4", "body", "caption"]).optional() }).strict(),
};
const TextFieldApi = {
  name: "TextField",
  schema: z
    .object({ label: CommonSchemas.DynamicString, value: CommonSchemas.DynamicString, action: CommonSchemas.Action.optional() })
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

/** Enfants empilés. */
const Column = createComponentImplementation(ColumnApi, ({ props, buildChild }) => (
  <div className="a2ui-column">
    {Array.isArray(props.children) &&
      props.children.map((ref) =>
        typeof ref === "string"
          ? <div key={ref} className="a2ui-slot">{buildChild(ref)}</div>
          : <div key={`${ref.id}-${ref.basePath}`} className="a2ui-slot">{buildChild(ref.id, ref.basePath)}</div>,
      )}
  </div>
));

/** Un post-it autour d'un seul enfant. */
const Card = createComponentImplementation(CardApi, ({ props, buildChild }) => (
  <div className="postit panel-card">{buildChild(props.child)}</div>
));

/** Titre (h4), texte courant ou petite légende d'un post-it. */
const Text = createComponentImplementation(TextApi, ({ props }) => (
  <span className={props.variant === "h4" ? "postit-title" : props.variant === "caption" ? "postit-hint" : undefined}>{props.text}</span>
));

/** Champ d'un post-it : la valeur et l'action partent à la sortie ou sur Entrée. */
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
    <label className="note-field">
      <span className="postit-hint">{props.label}</span>
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
    </label>
  );
});

/** Petit compteur − valeur + ; l'action part à chaque pas. */
const Stepper = createComponentImplementation(StepperApi, ({ props }) => {
  const { label, value, min = 1, max = 24 } = props;
  const step = (next: number) => {
    props.setValue(next);
    props.action?.();
  };
  return (
    <div className="stepper">
      <span className="postit-hint">{label}</span>
      <button aria-label={t("ui-stepper-less", { label })} disabled={value <= min} onClick={() => step(value - 1)}>−</button>
      <span className="stepper-value">{value}</span>
      <button aria-label={t("ui-stepper-more", { label })} disabled={value >= max} onClick={() => step(value + 1)}>+</button>
    </div>
  );
});

/** Le catalogue des surfaces d'Écho. */
export const panelCatalog = new Catalog(CATALOG_ID, [Column, Card, Text, TextField, Stepper]);
