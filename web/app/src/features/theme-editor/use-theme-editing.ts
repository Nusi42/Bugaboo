import type { Schema } from "@bugaboo/api-client";
import type { ThemeRead } from "@bugaboo/wasm";
import { useMemo, useState } from "react";

import type { EditLock } from "../../shared/editing/index.ts";
import { isColorToken, sameValues } from "./model.ts";
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
  /** Whether the name or the values differ from what was saved. */
  readonly dirty: boolean;
  /** Whether the person holds the lock, so the theme can be changed. */
  readonly mine: boolean;
  /** The rating of `bb-core`; none until the module is there. */
  readonly read: ThemeRead | null;
  /** Whether `bb-core` takes these values (a save would). */
  readonly accepts: (values: ThemeValues) => boolean;
  /** Whether a save of a theme for everyone was refused for colors that are still hard to read. */
  readonly refused: boolean;
  /** A save of a theme for everyone was refused for its colors that are hard to read. */
  readonly refuse: () => void;
  /** How many colors are hard to read: a save asks or refuses while there are any. */
  readonly hardCount: number;
  /** Opens a color that is hard to read and puts the cursor in it. */
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
  const [refused, setRefused] = useState(false);
  const mine = lock.state.phase === "mine";
  const reader = useThemeReader();
  const read = useMemo(
    () => reader?.(theme.extends, values) ?? null,
    [reader, theme.extends, values],
  );
  // A refusal holds until the colors it named read well; a later color that is hard to read is
  // not refused again until the next save.
  if (refused && read !== null && read.hard_colors.length === 0) {
    setRefused(false);
  }
  const shown = useShownColors();
  const kept = useKeptDraft(theme.id, detail.values, setValues);
  return {
    saved: detail.values,
    values,
    setValues,
    name,
    setName,
    dirty: name !== theme.name || !sameValues(values, detail.values),
    mine,
    read,
    accepts: (next) => reader !== undefined && reader(theme.extends, next) !== null,
    hardCount: read?.hard_colors.length ?? 0,
    refused,
    refuse: () => {
      setRefused(true);
    },
    show: (place) => {
      if (isColorToken(place)) {
        shown.reveal(place);
      }
    },
    shown,
    kept,
    draftShown: useDraftPreview(
      theme.id,
      values,
      theme.version,
      mine && kept.read && !sameValues(values, detail.values),
    ),
  };
}
