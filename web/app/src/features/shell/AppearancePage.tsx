import { t } from "@bugaboo/messages";
import {
  Card,
  ChoiceGroup,
  Switch,
  useAppearance,
  type Choice,
  type DarkThemeId,
  type LightThemeId,
  type SpacingChoice,
} from "@bugaboo/ui";

import { usePageTitle } from "../../shared/use-page.ts";
import styles from "./AppearancePage.module.css";
import { themeChoiceLabels, themeLabels } from "./theme-labels.ts";

const lightChoices: readonly Choice<LightThemeId>[] = [
  { value: "daylight", label: themeLabels.daylight },
  { value: "paper", label: themeLabels.paper },
];

const darkChoices: readonly Choice<DarkThemeId>[] = [
  { value: "dusk", label: themeLabels.dusk },
  { value: "midnight", label: themeLabels.midnight },
];

const spacingChoices: readonly Choice<SpacingChoice>[] = [
  { value: "comfortable", label: t("appearance.spacing.comfortable") },
  { value: "compact", label: t("appearance.spacing.compact") },
];

/** The theme, and the light and the dark theme that *Match my device* uses (THM-01, THM-02). */
function Themes() {
  const { appearance, deviceTheme, change } = useAppearance();
  return (
    <>
      <p className={styles.help}>{t("setting.appearance.theme.help")}</p>
      <ChoiceGroup
        legend={t("theme.legend")}
        choices={themeChoiceLabels(deviceTheme)}
        value={appearance.theme}
        onChange={(theme) => {
          change({ theme });
        }}
      />
      <p className={styles.help}>{t("appearance.device-help")}</p>
      <ChoiceGroup
        legend={t("appearance.light")}
        choices={lightChoices}
        value={appearance.light}
        onChange={(light) => {
          change({ light });
        }}
      />
      <ChoiceGroup
        legend={t("appearance.dark")}
        choices={darkChoices}
        value={appearance.dark}
        onChange={(dark) => {
          change({ dark });
        }}
      />
    </>
  );
}

/** Spacing, text size and motion (THM-06). */
function Comfort() {
  const { appearance, change } = useAppearance();
  return (
    <>
      <ChoiceGroup
        legend={t("appearance.spacing.legend")}
        choices={spacingChoices}
        value={appearance.spacing}
        onChange={(spacing) => {
          change({ spacing });
        }}
      />
      <Switch
        label={t("appearance.larger-text")}
        isSelected={appearance.largerText}
        onChange={(largerText) => {
          change({ largerText });
        }}
      />
      <Switch
        label={t("appearance.less-motion")}
        isSelected={appearance.lessMotion}
        onChange={(lessMotion) => {
          change({ lessMotion });
        }}
      />
      <p className={styles.help}>{t("appearance.device-settings")}</p>
    </>
  );
}

/** *My settings › Appearance* (CFG-04, THM-01, THM-02, THM-06): the choices live in this browser. */
export function AppearancePage() {
  usePageTitle(t("appearance.title"));
  return (
    <Card heading={t("appearance.title")} level={1}>
      <Themes />
      <Comfort />
    </Card>
  );
}
