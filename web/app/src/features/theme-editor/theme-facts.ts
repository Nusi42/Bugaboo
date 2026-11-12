import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { themeIds } from "@bugaboo/ui";

import { themeLabels } from "../../shared/theme-labels.ts";

/** The name of the built-in theme an own theme starts from (*Dusk*); its ID for one this app does not know. */
export function startsFromName(extendsTheme: string): string {
  const known = themeIds.find((id) => id === extendsTheme);
  return known === undefined ? extendsTheme : themeLabels[known];
}

/**
 * The facts of an own theme in a list (UX §5.6): the built-in theme it started from, light or dark,
 * and whether all its text is readable as saved, or that it was never saved.
 */
export function themeFacts(theme: Schema<"ThemeBody">): string {
  const startsFrom = startsFromName(theme.extends);
  const look = theme.light ? t("theme-editor.light") : t("theme-editor.dark");
  if (!theme.saved) {
    return t("theme-editor.facts-unsaved", { theme: startsFrom, look });
  }
  return t("theme-editor.facts", {
    theme: startsFrom,
    look,
    hard: theme.readability.colors_hard_to_read,
  });
}
