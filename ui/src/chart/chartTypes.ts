// Data shapes shared by the catalog's zod schemas (`a2ui/dataComponents.tsx`) and the op types
// (`slideTypes.ts`): each is defined once, here.

import { z } from "zod";

/** A dataset.source cell: number, text, checkbox (a boolean) or empty. */
export const CellSchema = z.union([z.number(), z.string(), z.boolean(), z.null()]);
export type Cell = z.infer<typeof CellSchema>;

/** The data a view shows, and how many views share it (this one included). */
export const DataRefSchema = z
  .object({ id: z.string(), name: z.string(), label: z.string().optional(), users: z.number().int().nonnegative() })
  .strict();
export type DataRef = z.infer<typeof DataRefSchema>;

/** A data the view can switch to; ids are unique, so a picked one names a single data. */
export const DataChoicesSchema = z
  .array(z.object({ id: z.string(), name: z.string(), label: z.string().optional() }).strict())
  .readonly()
  .refine((cs) => new Set(cs.map((c) => c.id)).size === cs.length, { message: "data choice ids must be unique" });
export type DataChoice = z.infer<typeof DataChoicesSchema>[number];
