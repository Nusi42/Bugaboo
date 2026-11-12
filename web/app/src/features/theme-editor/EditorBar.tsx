import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

import type { EditLock } from "../../shared/editing/index.ts";
import { messageOf } from "../../shared/errors.ts";
import { AskToSave } from "./AskToSave.tsx";
import { ThemeActions } from "./ThemeActions.tsx";
import { useSaving } from "./use-save-theme.ts";
import type { Editing } from "./use-theme-editing.ts";

export interface EditorBarProps {
  readonly detail: Schema<"ThemeDetailBody">;
  readonly editing: Editing;
  readonly lock: EditLock;
  /** What a phone adds beside *Save theme*: the button that opens the preview. */
  readonly extra?: ReactNode;
}

/**
 * The one bar at the foot of the editor (UX §5.6): *Save a copy* and *Download* of the saved theme
 * on the start side, *Throw my changes away* and *Save theme* on the end side (and *Preview* on a
 * phone, where the bar stays at the foot of the screen). A personal theme with colors that are hard
 * to read asks *Save this theme?* first; a theme for everyone sends the cursor to the readability.
 */
export function EditorBar({ detail, editing, lock, extra }: EditorBarProps) {
  const { dirty } = editing;
  const { theme } = detail;
  const saving = useSaving(detail, editing, lock);
  return (
    <>
      {saving.error === null || saving.error === undefined ? null : (
        <Alert tone={alertTone.danger}>{messageOf(saving.error)}</Alert>
      )}
      <div {...partOf(part.themeEditorBar)}>
        <div {...partOf(part.row)}>
          <ThemeActions theme={theme} />
        </div>
        <div {...partOf(part.row)}>
          {extra}
          <Button
            look={buttonLook.quiet}
            isDisabled={!dirty}
            onPress={() => {
              editing.setValues(detail.values);
              editing.setName(theme.name);
            }}
          >
            {t("editing.changes-drop")}
          </Button>
          <Button
            look={buttonLook.primary}
            isPending={saving.pending}
            isDisabled={!dirty}
            onPress={saving.save}
          >
            {t("theme-editor.save")}
          </Button>
        </div>
      </div>
      <AskToSave saving={saving} hard={editing.read?.hard_colors ?? []} />
    </>
  );
}
