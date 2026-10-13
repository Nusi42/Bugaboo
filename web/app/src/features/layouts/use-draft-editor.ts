import type { Schema } from "@bugaboo/api-client";
import { useLayout } from "@bugaboo/ui";
import { useState } from "react";

import { actionsOn, moveLabel, type EditorActions, type EditorContext } from "./editor-actions.ts";
import { addSection, bodyOf, draftOf, sameLayout, type Draft } from "./editor-model.ts";
import { useSaveLayout } from "./layouts.ts";
import type { LayoutLock } from "./use-layout-lock.ts";

/** What the screen of the editor gets: the draft, what it is changed with, and the save. */
export interface DraftEditorState {
  readonly draft: Draft;
  readonly context: EditorContext;
  readonly actions: EditorActions;
  /** Whether the editing is on: the person holds the lock. */
  readonly mine: boolean;
  readonly dirty: boolean;
  readonly canSave: boolean;
  readonly pending: boolean;
  readonly error: unknown;
  readonly addSection: () => void;
  readonly save: () => void;
  readonly discard: () => void;
}

/** The IDs of the roles of the editor: a field that is added starts with all of them ticked. */
function everyone(editor: Schema<"LayoutEditorBody">): readonly string[] {
  return editor.roles.map((role) => role.id);
}

/** What the parts of the editor know about the editor they are in. */
function contextOf(
  editor: Schema<"LayoutEditorBody">,
  draft: Draft,
  canDrag: boolean,
): EditorContext {
  return {
    kind: editor.kind,
    roles: editor.roles,
    canDrag,
    moves: draft.sections.map((section, at) => ({
      value: section.key,
      label: moveLabel(at + 1, section.title),
    })),
  };
}

/**
 * The state of the editor of one layout: the draft that holds everything the person changed until
 * *Save layout*, and the save, which is made on the version the editor looked at and under the
 * lock the person holds. After a save the server has let go of the lock, so it is asked for again.
 */
export function useDraftEditor(
  editor: Schema<"LayoutEditorBody">,
  lock: LayoutLock,
  onSaved: (version: number | undefined) => void,
): DraftEditorState {
  const [draft, setDraft] = useState(() => draftOf(editor));
  const send = useSaveLayout();
  const phone = useLayout() === "phone";
  const slot = editor.layout ?? null;
  const { state } = lock;
  const mine = state.phase === "mine";
  return {
    draft,
    mine,
    dirty: !sameLayout(draft, draftOf(editor)),
    canSave: slot !== null && mine,
    pending: send.isPending,
    error: send.error,
    context: contextOf(editor, draft, !phone && mine),
    actions: actionsOn(setDraft, everyone(editor)),
    addSection: () => {
      setDraft(addSection);
    },
    save: () => {
      if (slot !== null && state.phase === "mine") {
        send.mutate(
          {
            layout: slot.id,
            version: slot.version,
            lockToken: state.token,
            sections: bodyOf(draft),
          },
          {
            onSuccess: (saved) => {
              onSaved(saved.layout?.version);
            },
            onSettled: lock.retry,
          },
        );
      }
    },
    discard: () => {
      setDraft(draftOf(editor));
    },
  };
}
