import type { Schema } from "@bugaboo/api-client";
import type { ThemeRead } from "@bugaboo/wasm";
import { useMemo, useState } from "react";

import type { EditLock } from "../../shared/editing/index.ts";
import { hardPairsOf, isColorToken, type NamedPair } from "./model.ts";
import type { ThemeValues } from "./themes.ts";
import { useDraftPreview } from "./use-draft-preview.ts";
import { useKeptDraft, type KeptDraft } from "./use-kept-draft.ts";
import { useShownColors, type ShownColors } from "./use-shown-colors.ts";
import { useThemeReader } from "./use-theme-reader.ts";

/** Everything the screen of the editor shows and changes. */
export interface Editing {
  /** The values as they were saved: the editor starts from them and *Throw my changes away* goes back. */
  readonly saved: ThemeValues;
  readonly values: ThemeValues;
  readonly setValues: (values: ThemeValues) => void;
  readonly name: string;
  readonly setName: (name: string) => void;
  /** Whether the person holds the lock, so the theme can be changed. */
  readonly mine: boolean;
  /** The rating of `bb-core`; none until the module is there. */
  readonly read: ThemeRead | null;
  /** Whether `bb-core` takes these values (a save would). */
  readonly accepts: (values: ThemeValues) => boolean;
  readonly hard: readonly NamedPair[];
  /** *Show* of a pair that is hard to read: the color to change to mend it. */
  readonly show: (place: string) => void;
  readonly shown: ShownColors;
  readonly kept: KeptDraft;
  readonly draftShown: boolean;
}

/** The state of the theme editor of one theme, under the lock the person holds. */
export function useThemeEditing(detail: Schema<"ThemeDetailBody">, lock: EditLock): Editing {
  const { theme } = detail;
  const [values, setValues] = useState<ThemeValues>(detail.values);
  const [name, setName] = useState(theme.name);
  const mine = lock.state.phase === "mine";
  const reader = useThemeReader();
  const read = useMemo(
    () => reader?.(theme.extends, values) ?? null,
    [reader, theme.extends, values],
  );
  const shown = useShownColors();
  return {
    saved: detail.values,
    values,
    setValues,
    name,
    setName,
    mine,
    read,
    accepts: (next) => reader !== undefined && reader(theme.extends, next) !== null,
    hard: hardPairsOf(read?.pairs ?? []),
    show: (place) => {
      if (isColorToken(place)) {
        shown.reveal(place);
      }
    },
    shown,
    kept: useKeptDraft(theme.id, detail.values, setValues),
    draftShown: useDraftPreview(theme.id, values, theme.version, mine),
  };
}
