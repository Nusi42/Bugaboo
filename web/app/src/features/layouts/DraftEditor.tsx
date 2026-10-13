import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Button, partOf, part } from "@bugaboo/ui";

import { AvailableFields } from "./AvailableFields.tsx";
import { SaveBar } from "./SaveBar.tsx";
import { SectionCard } from "./SectionCard.tsx";
import { useDraftEditor } from "./use-draft-editor.ts";
import type { LayoutLock } from "./use-layout-lock.ts";

export interface DraftEditorProps {
  readonly editor: Schema<"LayoutEditorBody">;
  readonly lock: LayoutLock;
}

/**
 * The layout being changed: sections of fields, each field with the roles it is shown to, and the
 * fields that are on no section. Everything is kept in the draft until *Save layout*. Without the
 * lock the whole editor is switched off, and the draft stays for when the lock comes back.
 */
export function DraftEditor({ editor, lock }: DraftEditorProps) {
  const state = useDraftEditor(editor, lock);
  const { draft, context, actions } = state;
  return (
    <fieldset disabled={!state.mine} {...partOf(part.layoutEditor)}>
      {draft.sections.map((section, at) => (
        <SectionCard
          key={section.key}
          section={section}
          number={at + 1}
          count={draft.sections.length}
          context={context}
          actions={actions}
        />
      ))}
      <div {...partOf(part.row)}>
        <Button onPress={state.addSection}>{t("layouts.add-section")}</Button>
      </div>
      <AvailableFields rows={draft.available} context={context} actions={actions} />
      <SaveBar
        dirty={state.dirty}
        canSave={state.canSave}
        pending={state.pending}
        saved={state.saved}
        error={state.error}
        onSave={state.save}
        onDiscard={state.discard}
      />
    </fieldset>
  );
}
