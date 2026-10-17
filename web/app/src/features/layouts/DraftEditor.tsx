import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Button, partOf, part } from "@bugaboo/ui";

import { SaveBar, shownSaveError, type EditLock } from "../../shared/editing/index.ts";
import { AvailableFields } from "./AvailableFields.tsx";
import { KeptDraft } from "./KeptDraft.tsx";
import { SectionCard } from "./SectionCard.tsx";
import { useDraftEditor } from "./use-draft-editor.ts";

export interface DraftEditorProps {
  readonly editor: Schema<"LayoutEditorBody">;
  readonly lock: EditLock;
  /** The layout on screen is the one that was just saved. */
  readonly saved: boolean;
  /** A save worked; says the version that was saved. */
  readonly onSaved: (version: number | undefined) => void;
}

/**
 * The layout being changed: sections of fields, each field with the roles it is shown to, and the
 * fields that are on no section. Everything is kept in the draft until *Save layout*. Without the
 * lock the whole editor is switched off, and the draft stays for when the lock comes back.
 */
export function DraftEditor({ editor, lock, saved, onSaved }: DraftEditorProps) {
  const state = useDraftEditor(editor, lock, onSaved);
  const { draft, context, actions } = state;
  return (
    <>
      <KeptDraft editor={editor} layout={editor.layout?.id ?? ""} onRestore={state.restore} />
      <fieldset disabled={!state.mine} {...partOf(part.layoutEditor)}>
        {editor.choose_roles ? null : <p {...partOf(part.muted)}>{t("layouts.roles-hidden")}</p>}
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
          saved={saved}
          error={shownSaveError(state.error, lock.state)}
          onSave={state.save}
          onDiscard={state.discard}
          saveLabel={t("layouts.save")}
        />
      </fieldset>
    </>
  );
}
