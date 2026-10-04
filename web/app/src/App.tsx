import { t } from "@bugaboo/messages";
import { ThemePicker, useTheme, type ThemeChoice } from "@bugaboo/ui";

import styles from "./App.module.css";

const themeLabels = {
  daylight: t("theme.daylight"),
  paper: t("theme.paper"),
  dusk: t("theme.dusk"),
  midnight: t("theme.midnight"),
} as const;

export function App() {
  const { choice, resolved, deviceTheme, choose } = useTheme();
  const labels: Readonly<Record<ThemeChoice, string>> = {
    ...themeLabels,
    device: t("theme.device-resolved", {
      device: t("theme.device"),
      theme: themeLabels[deviceTheme],
    }),
  };
  return (
    <main className={styles.page}>
      <h1 className={styles.brand}>{t("hello.title")}</h1>
      <p className={styles.intro}>{t("hello.intro")}</p>
      <ThemePicker legend={t("theme.legend")} labels={labels} value={choice} onChange={choose} />
      <p className={styles.current}>{t("hello.current", { theme: themeLabels[resolved] })}</p>
    </main>
  );
}
