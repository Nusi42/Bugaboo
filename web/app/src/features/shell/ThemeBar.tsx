import { t } from "@bugaboo/messages";
import { ThemePicker, useTheme, type ThemeChoice } from "@bugaboo/ui";

const themeLabels = {
  daylight: t("theme.daylight"),
  paper: t("theme.paper"),
  dusk: t("theme.dusk"),
  midnight: t("theme.midnight"),
} as const;

/** The choice of theme that is at the bottom of every page: it works before anybody signs in. */
export function ThemeBar() {
  const { choice, deviceTheme, choose } = useTheme();
  const labels: Readonly<Record<ThemeChoice, string>> = {
    ...themeLabels,
    device: t("theme.device-resolved", {
      device: t("theme.device"),
      theme: themeLabels[deviceTheme],
    }),
  };
  return (
    <ThemePicker legend={t("theme.legend")} labels={labels} value={choice} onChange={choose} />
  );
}
