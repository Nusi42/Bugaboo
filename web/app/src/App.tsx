import { t } from "@bugaboo/messages";
import { ThemePicker, useTheme, type ThemeChoice } from "@bugaboo/ui";

import styles from "./App.module.css";

const labels: Readonly<Record<ThemeChoice, string>> = {
  device: t("theme.device"),
  daylight: t("theme.daylight"),
  paper: t("theme.paper"),
  dusk: t("theme.dusk"),
  midnight: t("theme.midnight"),
};

export function App() {
  const [choice, choose] = useTheme();
  return (
    <main className={styles.page}>
      <h1 className={styles.brand}>{t("hello.title")}</h1>
      <p className={styles.intro}>{t("hello.intro")}</p>
      <ThemePicker legend={t("theme.legend")} labels={labels} value={choice} onChange={choose} />
      <p className={styles.current}>{t("hello.current", { theme: labels[choice] })}</p>
    </main>
  );
}
