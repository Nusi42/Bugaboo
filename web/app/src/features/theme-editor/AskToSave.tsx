import { t } from "@bugaboo/messages";
import { AskDialog, buttonLook } from "@bugaboo/ui";
import type { HardColor } from "@bugaboo/wasm";

import { listText } from "../../shared/list-text.ts";
import { colorName } from "./model.ts";
import type { Saving } from "./use-save-theme.ts";

/**
 * *Save this theme?* of a personal theme with colors that are hard to read (UX §5.6): it names
 * them, and says that only its maker sees the theme.
 */
export function AskToSave({
  saving,
  hard,
}: {
  readonly saving: Saving;
  readonly hard: readonly HardColor[];
}) {
  const names = hard.map((color) => colorName(color.place));
  const asking = saving.question === "ask";
  return (
    <AskDialog
      isOpen={asking}
      onOpenChange={(open) => {
        if (!open) {
          saving.close();
        }
      }}
      title={t("theme-editor.ask-title")}
      text={t("theme-editor.ask", { count: names.length, colors: listText(names) })}
      confirmLabel={t("theme-editor.save-anyway")}
      cancelLabel={t("common.cancel")}
      confirmLook={buttonLook.primary}
      onConfirm={saving.send}
    />
  );
}
