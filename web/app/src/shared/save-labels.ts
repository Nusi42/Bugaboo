import { t } from "@bugaboo/messages";
import type { SaveStatusProps } from "@bugaboo/ui";

import { formatTime } from "./format.ts";

/**
 * The three words next to a change (*Saving…*, *Saved*, *Not saved yet*), the same on every
 * screen. `savedAt` adds the time to *Saved* where the screen knows it.
 */
export function saveLabels(savedAt: string | null = null): SaveStatusProps["labels"] {
  return {
    saving: t("save.saving"),
    saved: savedAt === null ? t("save.saved") : t("save.saved-at", { time: formatTime(savedAt) }),
    "not-saved": t("save.not-saved"),
  };
}
