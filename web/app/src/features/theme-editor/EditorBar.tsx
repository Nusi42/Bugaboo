import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, partOf, part, useLayout } from "@bugaboo/ui";
import { useState, type ReactNode } from "react";

import type { EditLock } from "../../shared/editing/index.ts";
import { messageOf } from "../../shared/errors.ts";
import { AskToSave } from "./AskToSave.tsx";
import { ActionFailure, ThemeActions } from "./ThemeActions.tsx";
import { useSaving } from "./use-save-theme.ts";
import type { Editing } from "./use-theme-editing.ts";

export interface EditorBarProps {
  readonly detail: Schema<"ThemeDetailBody">;
  readonly editing: Editing;
  readonly lock: EditLock;
  /** What a phone adds beside *Save theme*: the button that opens the preview. */
  readonly extra?: ReactNode;
  /** *Not saved yet*, beside *Save theme*, in a place that is always there (UX §6.4). */
  readonly note: ReactNode;
}

/**
 * The end of the bar: the note, what a phone adds, *Throw my changes away* and *Save theme*. A theme
 * for everyone whose save was refused for colors that are hard to read cannot be saved until they
 * read well: *Save theme* is off, and the readability on top says why.
 */
function BarEnd({
  detail,
  editing,
  saving,
  extra,
  note,
}: Pick<EditorBarProps, "detail" | "editing" | "extra" | "note"> & {
  readonly saving: ReturnType<typeof useSaving>;
}) {
  return (
    <div {...partOf(part.themeEditorBarEnd)}>
      {note}
      {extra}
      <Button
        look={buttonLook.quiet}
        isDisabled={!editing.dirty}
        onPress={() => {
          editing.setValues(detail.values);
          editing.setName(detail.theme.name);
        }}
      >
        {t("editing.changes-drop")}
      </Button>
      <Button
        look={buttonLook.primary}
        isPending={saving.pending}
        isDisabled={!editing.dirty || editing.refused}
        onPress={saving.save}
      >
        {t("theme-editor.save")}
      </Button>
    </div>
  );
}

/**
 * The one bar at the foot of the editor (UX §5.6): *Save a copy* and *Download* of the saved theme
 * on the start side, *Throw my changes away* and *Save theme* on the end side. On a phone the bar
 * stays at the foot of the screen with *Preview*, *Throw my changes away* and *Save theme* only;
 * *Save a copy* and *Download* stand in the page above it. A personal theme with colors that are hard
 * to read asks *Save this theme?* first; a theme for everyone sends the cursor to the readability.
 */
export function EditorBar({ detail, editing, lock, extra, note }: EditorBarProps) {
  const { theme } = detail;
  const saving = useSaving(detail, editing, lock);
  // On a phone *Save a copy* and *Download* stand in the page; the pinned bar keeps the rest.
  const phone = useLayout() === "phone";
  const [failure, setFailure] = useState<unknown>(null);
  return (
    <>
      {saving.error === null || saving.error === undefined ? null : (
        <Alert tone={alertTone.danger}>{messageOf(saving.error)}</Alert>
      )}
      {phone ? (
        <div {...partOf(part.row)}>
          <ThemeActions theme={theme} look={buttonLook.plain} onFailed={setFailure} />
        </div>
      ) : null}
      <div {...partOf(part.themeEditorBar)}>
        {phone ? null : (
          <div {...partOf(part.row)}>
            <ThemeActions theme={theme} onFailed={setFailure} />
          </div>
        )}
        <BarEnd detail={detail} editing={editing} saving={saving} extra={extra} note={note} />
      </div>
      <ActionFailure failure={failure} />
      <AskToSave saving={saving} hard={editing.read?.hard_colors ?? []} />
    </>
  );
}
