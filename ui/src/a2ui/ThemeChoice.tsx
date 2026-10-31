// Carte d'un thème du sélecteur de l'hôte : la slide rendue dans ce thème (image), ses pastilles,
// son nom. Survol ou focus : aperçu sur la scène ; clic : l'action `theme`.
// With quick hints on, its digit 1–4, the one `quickAction` reads, kept current as the surface changes.

import { useCallback, useContext, useEffect, useMemo, useReducer } from "react";
import { z } from "zod";
import { createComponentImplementation } from "@a2ui/react/v0_9";
import { CommonSchemas } from "@a2ui/web_core/v0_9";
import type { ComponentModel, Subscription, SurfaceComponentsModel } from "@a2ui/web_core/v0_9";
import type { Component } from "../agent/a2uiModel";
import { quickDigit } from "../agent/quick";
import { QuickHintsContext } from "./quickHints";
import { Hex } from "./schemas";
import { StagePreviewContext } from "./stagePreview";
import { svgSrc } from "./svgSrc";

/** Taille maximale de la miniature (miroir de `THUMB_BYTES`, oku-slides-session). */
const SVG_MAX = 512 * 1024;

/** Schéma de `ThemeChoice` : l'hôte envoie toujours des littéraux, jamais de liaison. */
export const ThemeChoiceApi = {
  name: "ThemeChoice",
  schema: z
    .object({
      svg: z.string().max(SVG_MAX).optional(),
      name: z.string().max(200),
      slide: z.string(),
      swatches: z.array(Hex).readonly(),
      active: z.boolean(),
      action: CommonSchemas.Action.optional(),
    })
    .strict(),
};

/** Une carte de thème : survol ou focus posent l'aperçu, que seule elle retire. */
export const ThemeChoice = createComponentImplementation(ThemeChoiceApi, ({ props, context }) => {
  const store = useContext(StagePreviewContext);
  const hints = useContext(QuickHintsContext);
  useSurfaceChanges(context.surfaceComponents, hints);
  const digit = hints ? quickDigit(surfaceComponents(context.surfaceComponents.values), context.componentModel.id) : null;
  const src = useMemo(() => (props.svg ? svgSrc(props.svg) : null), [props.svg]);
  const preview = useMemo(() => (src ? { slide: props.slide, url: src } : null), [props.slide, src]);
  const show = () => {
    if (preview) store?.show(preview);
  };
  const hide = useCallback(() => {
    if (preview) store?.hide(preview);
  }, [store, preview]);
  // Démontée pendant le survol (surface remplacée) : son aperçu ne reste pas sur la scène.
  useEffect(() => hide, [hide]);
  return (
    <button
      type="button"
      className="theme-choice"
      aria-pressed={props.active}
      onClick={() => props.action?.()}
      onPointerEnter={show}
      onPointerLeave={hide}
      onFocus={show}
      onBlur={hide}
    >
      {src ? <img src={src} alt="" draggable={false} /> : <span className="theme-choice-blank" aria-hidden="true" />}
      <span className="theme-choice-swatches" aria-hidden="true">
        {props.swatches.map((c, i) => (
          <span key={i} className="a2ui-swatch" style={{ background: c }} />
        ))}
      </span>
      <span className="caption">
        {digit !== null && <><span className="chip-key" aria-hidden="true">{digit}</span> </>}
        <bdi>{props.name}</bdi>
      </span>
    </button>
  );
});

/**
 * Re-renders the card whenever the surface gains, loses or updates a component, while `on`.
 *
 * Another component's change can shift the card's digit (t-1389).
 */
function useSurfaceChanges(model: SurfaceComponentsModel, on: boolean): void {
  const [, bump] = useReducer((n: number) => n + 1, 0);
  useEffect(() => {
    if (!on) return;
    const updates = new Map<string, Subscription>();
    const watch = (c: ComponentModel) => {
      updates.get(c.id)?.unsubscribe();
      updates.set(c.id, c.onUpdated.subscribe(bump));
    };
    for (const c of model.values) watch(c);
    const created = model.onCreated.subscribe((c) => {
      watch(c);
      bump();
    });
    const deleted = model.onDeleted.subscribe((id) => {
      updates.get(id)?.unsubscribe();
      updates.delete(id);
      bump();
    });
    return () => {
      created.unsubscribe();
      deleted.unsubscribe();
      for (const u of updates.values()) u.unsubscribe();
    };
  }, [model, on]);
}

/** The surface's components in `quickDigit`'s shape: `component` is the type. */
function surfaceComponents(models: Iterable<{ id: string; type: string; properties: Record<string, unknown> }>): Component[] {
  return [...models].map((m) => ({ ...m.properties, id: m.id, component: m.type }));
}
