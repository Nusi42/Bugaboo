import type { Schema } from "@bugaboo/api-client";
import { Alert, alertTone, buttonLook, partOf, part, useLayout } from "@bugaboo/ui";
import { useState, type ReactNode } from "react";

import type { EditLock } from "../../shared/editing/index.ts";
import { messageOf } from "../../shared/errors.ts";
import { AskToSave } from "./AskToSave.tsx";
import { BarEnd } from "./BarEnd.tsx";
import { ActionFailure, ThemeActions } from "./ThemeActions.tsx";
import { useSaving } from "./use-save-theme.ts";
import type { Editing } from "./use-theme-editing.ts";

export interface EditorBarProps {
  readonly detail: Schema<"ThemeDetailBody">;
  readonly editing: Editing;
  readonly lock: EditLock;
  /** The save of the theme, kept by the page. */
  readonly put: Parameters<typeof useSaving>[3];
  /** What a phone adds beside *Save theme*: the button that opens the preview. */
  readonly extra?: ReactNode;
}

/**
 * The one bar at the foot of the editor (UX §5.6): *Save a copy* and *Download* of the saved theme
 * on the start side, *Throw my changes away* and *Save theme* on the end side. On a phone the bar
 * stays at the foot of the screen with *Preview*, *Throw my changes away* and *Save theme* only;
 * *Save a copy* and *Download* stand in the page above it. A personal theme with colors that are hard
 * to read asks *Save this theme?* first; a theme for everyone sends the cursor to the readability.
 */
export function EditorBar({ detail, editing, lock, put, extra }: EditorBarProps) {
  const { theme } = detail;
  const saving = useSaving(detail, editing, lock, put);
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
        <BarEnd detail={detail} editing={editing} saving={saving} extra={extra} />
      </div>
      <ActionFailure failure={failure} />
      <AskToSave saving={saving} hard={editing.read?.hard_colors ?? []} />
    </>
  );
}
