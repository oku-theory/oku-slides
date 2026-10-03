// Schémas zod partagés par les composants du catalogue.

import { z } from "zod";

/** Une couleur hexadécimale (#rgb, #rgba, #rrggbb, #rrggbbaa) : rien d'autre n'entre dans un style. */
export const Hex = z.string().regex(/^#([0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i);

/** La teinte d'une carte ou d'un badge. */
export const ToneSchema = z.enum(["neutral", "accent", "danger"]);

/** La classe CSS d'une teinte (aucune pour la teinte neutre), précédée d'une espace. */
export const toneClass = (tone: z.infer<typeof ToneSchema> | undefined) =>
  tone && tone !== "neutral" ? ` tone-${tone}` : "";
