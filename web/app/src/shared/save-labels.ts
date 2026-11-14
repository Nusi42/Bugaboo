import { t } from "@bugaboo/messages";
import type { SaveStatusProps } from "@bugaboo/ui";

import { formatTime } from "./format.ts";

/**
 * The three words next to a change (*Saving…*, *Saved*, *Not saved yet*), the same on every
 * screen. `savedAt` adds the time to *Saved* where the screen knows it.
 */
export function saveLabels(savedAt: string | null = null): Required<SaveStatusProps["labels"]> {
  return {
    saving: t("save.saving"),
    saved: savedAt === null ? t("save.saved") : t("save.saved-at", { time: formatTime(savedAt) }),
    "not-saved": t("save.not-saved"),
  };
}

/**
 * The words of a setting, which is sent the moment it is changed: *Saving…* and *Saved*, never
 * *Not saved yet*, so its place keeps no room for those.
 */
export function settingSaveLabels(): SaveStatusProps["labels"] {
  const { saving, saved } = saveLabels();
  return { saving, saved };
}
