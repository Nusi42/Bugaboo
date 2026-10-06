import { t } from "@bugaboo/messages";
import { ChoiceGroup, useAppearance } from "@bugaboo/ui";

import { themeChoiceLabels } from "../../shared/theme-labels.ts";

/** The choice of theme at the bottom of the pages before sign-in: it works before anybody signs in. */
export function ThemeBar() {
  const { appearance, deviceTheme, change } = useAppearance();
  return (
    <ChoiceGroup
      legend={t("theme.legend")}
      choices={themeChoiceLabels(deviceTheme)}
      value={appearance.theme}
      onChange={(theme) => {
        change({ theme });
      }}
    />
  );
}
