// Carte d'un thème du sélecteur de l'hôte : la slide rendue dans ce thème (image), ses pastilles,
// son nom. Survol ou focus : aperçu sur la scène ; clic : l'action `theme`.

import { useCallback, useContext, useEffect, useMemo } from "react";
import { z } from "zod";
import { createComponentImplementation } from "@a2ui/react/v0_9";
import { CommonSchemas } from "@a2ui/web_core/v0_9";
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
export const ThemeChoice = createComponentImplementation(ThemeChoiceApi, ({ props }) => {
  const store = useContext(StagePreviewContext);
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
      <bdi className="caption">{props.name}</bdi>
    </button>
  );
});
