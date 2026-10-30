import { settingKeys } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Card, ChoiceGroup, part, partOf, useAppearance, useLayout } from "@bugaboo/ui";

import { themeLabels } from "../../shared/theme-labels.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { SettingsForm, wordsOf, type ControlRenderer } from "../settings/index.ts";
import { ThemePicker } from "./ThemePicker.tsx";
import { usePersonalForm } from "./use-personal-form.ts";

/** The theme choices with their samples: the light and the dark theme that the choice above uses. */
const themePicker: ControlRenderer = (setting, onChange) => (
  <ThemePicker
    legend={wordsOf(setting.name_key)}
    choices={setting.choices.map((choice) => ({
      value: choice.value,
      label: wordsOf(choice.label_key),
    }))}
    value={String(setting.value)}
    onChange={onChange}
  />
);

/** What *Match my device* shows right now is part of its name, so the choice is never a guess. */
function useModePicker(): ControlRenderer {
  const { deviceTheme } = useAppearance();
  return (setting, onChange) => (
    <ChoiceGroup
      legend={wordsOf(setting.name_key)}
      choices={setting.choices.map((choice) => ({
        value: choice.value,
        label:
          choice.value === "device"
            ? t("theme.device-resolved", {
                device: t("theme.device"),
                theme: themeLabels[deviceTheme],
              })
            : wordsOf(choice.label_key),
      }))}
      value={String(setting.value)}
      onChange={onChange}
    />
  );
}

/**
 * *My settings › Appearance* (CFG-04, THM-01, THM-02, THM-06): *Match my device*, *Always light* or
 * *Always dark*, one light and one dark theme picked from samples, spacing, *Larger text*, *Less
 * motion* and where an issue opens. The choices are kept with the person on the server, so they
 * follow them to every browser; an administrator can lock a setting (*Set by your organization*).
 */
export function AppearancePage() {
  usePageTitle(t("appearance.title"));
  const form = usePersonalForm(["appearance."]);
  const modePicker = useModePicker();
  // A phone never opens an issue in a side panel (UX §3.4), so it is not asked where one opens.
  const phone = useLayout() === "phone";
  const settings = phone
    ? form.settings.filter((setting) => setting.key !== settingKeys.appearanceIssueView)
    : form.settings;
  return (
    <Card heading={t("appearance.title")} level={1}>
      <p {...partOf(part.text)}>{t("appearance.device-help")}</p>
      <SettingsForm
        {...form}
        settings={settings}
        searchable={false}
        controls={{
          [settingKeys.appearanceMode]: modePicker,
          [settingKeys.appearanceLightTheme]: themePicker,
          [settingKeys.appearanceDarkTheme]: themePicker,
        }}
      />
      <p {...partOf(part.text)}>{t("appearance.device-settings")}</p>
    </Card>
  );
}
