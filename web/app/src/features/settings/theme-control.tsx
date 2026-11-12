import { settingKeys, type Schema } from "@bugaboo/api-client";

import { ThemePicker } from "../../shared/ThemePicker.tsx";
import { ownThemeChoices, useOwnThemes } from "../theme-editor/index.ts";
import type { ControlRenderer } from "./SettingRow.tsx";
import { wordsOf } from "./texts.ts";

/**
 * The control of the settings *Light theme* and *Dark theme*: one theme picked from samples, the
 * built-in themes first (the choices of the setting), then the own themes of that look of `kinds`
 * (UX §4.13, ADR-0026 update M8): the organization's themes for everyone, and on a person's own
 * level their personal themes.
 */
export function useThemeControl(kinds: readonly Schema<"ThemeKind">[]): ControlRenderer {
  const themes = useOwnThemes().data?.themes ?? [];
  return (setting, onChange) => (
    <ThemePicker
      legend={wordsOf(setting.name_key)}
      choices={[
        ...setting.choices.map((choice) => ({
          value: choice.value,
          label: wordsOf(choice.label_key),
          look: choice.value,
        })),
        ...ownThemeChoices(themes, setting.key === settingKeys.appearanceLightTheme, kinds),
      ]}
      value={String(setting.value)}
      onChange={onChange}
    />
  );
}
