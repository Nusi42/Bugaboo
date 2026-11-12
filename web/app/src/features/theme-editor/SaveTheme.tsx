import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, AskDialog, buttonLook, partOf, part } from "@bugaboo/ui";

import { SaveBar, type EditLock } from "../../shared/editing/index.ts";
import { HardPairs } from "./ReadabilityLine.tsx";
import { ThemeActions } from "./ThemeActions.tsx";
import { sameValues } from "./model.ts";
import { useSaving } from "./use-save-theme.ts";
import type { Editing } from "./use-theme-editing.ts";

export interface SaveThemeProps {
  readonly detail: Schema<"ThemeDetailBody">;
  readonly editing: Editing;
  readonly lock: EditLock;
}

/**
 * The bottom of the editor: *Save*, *Throw my changes away*, what happened to the last save, the
 * pairs to fix before a theme for everyone can be saved, the question *Save anyway?* of a personal
 * theme, and *Save a copy* and *Download* of the saved theme.
 */
export function SaveTheme({ detail, editing, lock }: SaveThemeProps) {
  const { theme } = detail;
  const saving = useSaving(detail, editing, lock);
  const dirty = editing.name !== theme.name || !sameValues(editing.values, detail.values);
  const asking = saving.question === "ask";
  return (
    <>
      {saving.refused.length === 0 ? null : (
        <Alert tone={alertTone.danger}>
          <div {...partOf(part.stack)}>
            <span>{t("theme-editor.refused")}</span>
            <HardPairs pairs={saving.refused} onShow={editing.show} />
          </div>
        </Alert>
      )}
      <SaveBar
        dirty={dirty}
        canSave={editing.mine}
        pending={saving.pending}
        saved={saving.saved && !dirty}
        error={saving.error}
        onSave={saving.save}
        onDiscard={() => {
          editing.setValues(detail.values);
          editing.setName(theme.name);
        }}
        saveLabel={t("theme-editor.save")}
      />
      <ThemeActions theme={theme} />
      <AskDialog
        isOpen={asking}
        onOpenChange={(open) => {
          if (!open) {
            saving.close();
          }
        }}
        title={t("theme-editor.ask-title")}
        text={t("theme-editor.ask", { count: editing.hard.length })}
        confirmLabel={t("theme-editor.save-anyway")}
        cancelLabel={t("common.cancel")}
        confirmLook={buttonLook.primary}
        onConfirm={saving.send}
      />
    </>
  );
}
