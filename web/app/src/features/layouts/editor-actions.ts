import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import {
  editSection,
  nudgeField,
  nudgeSection,
  placeField,
  removeSection,
  setRole,
  unplaceField,
  type Draft,
  type Target,
} from "./editor-model.ts";
import type { MoveChoice } from "./FieldRow.tsx";
import type { FormKind } from "./layouts.ts";

/** Everything the editor can do to the draft of a layout, for the parts that draw it. */
export interface EditorActions {
  readonly editSection: (
    key: string,
    change: { readonly title?: string; readonly tab?: string },
  ) => void;
  readonly nudgeSection: (key: string, by: -1 | 1) => void;
  readonly removeSection: (key: string) => void;
  /** Puts a field (a row, or one that is available) into a section at a place. */
  readonly place: (field: string, target: Target) => void;
  readonly nudgeField: (field: string, by: -1 | 1) => void;
  readonly unplace: (field: string) => void;
  readonly setRole: (field: string, role: string, on: boolean) => void;
}

/** What the parts of the editor know about the editor they are in. */
export interface EditorContext {
  readonly kind: FormKind;
  readonly roles: readonly Schema<"EditorRoleBody">[];
  /** Whether fields can be dragged: not on a phone. */
  readonly canDrag: boolean;
  /** The sections a field can be moved to. */
  readonly moves: readonly MoveChoice[];
}

/** What the draft is changed with: every action of the editor, on the state of the screen. */
export function actionsOn(
  update: (change: (draft: Draft) => Draft) => void,
  everyone: readonly string[],
): EditorActions {
  return {
    editSection: (key, change) => {
      update((draft) => editSection(draft, key, change));
    },
    nudgeSection: (key, by) => {
      update((draft) => nudgeSection(draft, key, by));
    },
    removeSection: (key) => {
      update((draft) => removeSection(draft, key));
    },
    place: (field, target) => {
      update((draft) => placeField(draft, field, target, everyone));
    },
    nudgeField: (field, by) => {
      update((draft) => nudgeField(draft, field, by));
    },
    unplace: (field) => {
      update((draft) => unplaceField(draft, field));
    },
    setRole: (field, role, on) => {
      update((draft) => setRole(draft, field, role, on));
    },
  };
}

/** The title of a section for the lists that move a field: its number and, when it has one, its title. */
export function moveLabel(number: number, title: string): string {
  return title === ""
    ? t("layouts.section", { number: String(number) })
    : t("layouts.section-named", { number: String(number), title });
}
