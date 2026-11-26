import type { Schema } from "@bugaboo/api-client";

import type { ThemeRead } from "./readability.generated.ts";

// What the answers of the module are: each is checked against its shape before the app takes it, because a module
// that answers something else is a defect (see `broken` in `index.ts`).

/**
 * Rates an own theme that extends the built-in theme `extends` with `values`, as the server rates
 * it when it is saved; `null` for values a save would refuse.
 */
export type ThemeReader = (
  extendsTheme: string,
  values: Schema<"ThemeValuesBody">,
) => ThemeRead | null;

/** Whether the answer of the module is what the editor is told; anything else is a defect. */
export function isThemeRead(value: unknown): value is ThemeRead {
  return (
    typeof value === "object" &&
    value !== null &&
    "hard" in value &&
    Array.isArray(value.hard) &&
    "hard_colors" in value &&
    Array.isArray(value.hard_colors) &&
    "ratings" in value &&
    Array.isArray(value.ratings) &&
    "summary" in value &&
    typeof value.summary === "object" &&
    "colors" in value &&
    Array.isArray(value.colors) &&
    "upside_down" in value &&
    Array.isArray(value.upside_down)
  );
}

/** Where a box of a dashboard stands and how big it is, in columns and rows from 0 (`bb_core::dashboard::BoxPlace`). */
export interface BoxPlace {
  readonly column: number;
  readonly row: number;
  readonly width: number;
  readonly height: number;
}

/**
 * The places of the boxes of a dashboard packed upwards, as the server packs every change (`bb_core::dashboard::pack`):
 * in the same order, each moved up to the highest row where it meets no box above it.
 */
export type Packer = (places: readonly BoxPlace[]) => readonly BoxPlace[];
