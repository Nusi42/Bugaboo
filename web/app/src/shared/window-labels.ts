import { t } from "@bugaboo/messages";
import type { WindowLabels } from "@bugaboo/ui";

/** The words on the buttons of a window (UX §6.3), in the person's language. */
export function windowLabels(): WindowLabels {
  return {
    minimize: t("window.minimize"),
    maximize: t("window.maximize"),
    restoreDown: t("window.restore-down"),
    close: t("common.close"),
    back: t("shell.back"),
    resizeKeys: t("window.resize-keys"),
    draftKept: t("window.draft-kept"),
  };
}
