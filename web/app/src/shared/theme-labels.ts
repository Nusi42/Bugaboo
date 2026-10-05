import { t } from "@bugaboo/messages";
import { themeChoices, type Choice, type ThemeChoice, type ThemeId } from "@bugaboo/ui";

/** The names of the four built-in themes. */
export const themeLabels: Readonly<Record<ThemeId, string>> = {
  daylight: t("theme.daylight"),
  paper: t("theme.paper"),
  dusk: t("theme.dusk"),
  midnight: t("theme.midnight"),
};

/**
 * The choices of the theme: *Match my device* says which theme it shows right now, so the choice
 * is never a guess ("Match my device (Dusk)").
 */
export function themeChoiceLabels(deviceTheme: ThemeId): readonly Choice<ThemeChoice>[] {
  return themeChoices.map((value) => ({
    value,
    label:
      value === "device"
        ? t("theme.device-resolved", {
            device: t("theme.device"),
            theme: themeLabels[deviceTheme],
          })
        : themeLabels[value],
  }));
}
