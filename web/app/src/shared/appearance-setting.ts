import {
  settingChoices,
  settingDefaults,
  settingKeys,
  type SettingChoice,
} from "@bugaboo/api-client";
import { lightThemeIds, type Appearance } from "@bugaboo/ui";

import type { MySettings } from "./my-settings.ts";
import { flagOf, oneOf, textOf } from "./setting-values.ts";
import type { SettingChange } from "./setting-change.ts";

/** The theme choice: follow the device, or always light, or always dark. */
export type Mode = SettingChoice<"appearanceMode">;
/** Where an issue opens. */
export type IssueView = SettingChoice<"appearanceIssueView">;

/** What the person chose about how Bugaboo looks, as the settings hold it. */
export interface PersonalAppearance {
  readonly mode: Mode;
  readonly light: SettingChoice<"appearanceLightTheme">;
  readonly dark: SettingChoice<"appearanceDarkTheme">;
  readonly spacing: SettingChoice<"appearanceSpacing">;
  readonly lessMotionOn: boolean;
  readonly largerTextOn: boolean;
  readonly issueView: IssueView;
}

/** The look the settings describe; a value that is not a valid choice is the standard one. */
export function appearanceOf(settings?: MySettings): PersonalAppearance {
  return {
    mode: oneOf(
      settingChoices.appearanceMode,
      textOf(settings, settingKeys.appearanceMode, settingDefaults.appearanceMode),
      settingDefaults.appearanceMode,
    ),
    light: oneOf(
      settingChoices.appearanceLightTheme,
      textOf(settings, settingKeys.appearanceLightTheme, settingDefaults.appearanceLightTheme),
      settingDefaults.appearanceLightTheme,
    ),
    dark: oneOf(
      settingChoices.appearanceDarkTheme,
      textOf(settings, settingKeys.appearanceDarkTheme, settingDefaults.appearanceDarkTheme),
      settingDefaults.appearanceDarkTheme,
    ),
    spacing: oneOf(
      settingChoices.appearanceSpacing,
      textOf(settings, settingKeys.appearanceSpacing, settingDefaults.appearanceSpacing),
      settingDefaults.appearanceSpacing,
    ),
    lessMotionOn: flagOf(
      settings,
      settingKeys.appearanceLessMotion,
      settingDefaults.appearanceLessMotion,
    ),
    largerTextOn: flagOf(
      settings,
      settingKeys.appearanceLargerText,
      settingDefaults.appearanceLargerText,
    ),
    issueView: oneOf(
      settingChoices.appearanceIssueView,
      textOf(settings, settingKeys.appearanceIssueView, settingDefaults.appearanceIssueView),
      settingDefaults.appearanceIssueView,
    ),
  };
}

/** The look as the shell applies it: *Always light* and *Always dark* use their theme at any time. */
export function appearanceForShell(look: PersonalAppearance): Appearance {
  const theme = look.mode === "device" ? "device" : look.mode === "light" ? look.light : look.dark;
  return {
    theme,
    light: look.light,
    dark: look.dark,
    spacing: look.spacing,
    lessMotion: look.lessMotionOn,
    largerText: look.largerTextOn,
  };
}

/** The settings a change of the shell's look touches: what differs from the look as it is. */
export function changesForAppearance(
  before: Appearance,
  next: Appearance,
): readonly SettingChange[] {
  const changes: SettingChange[] = [];
  if (next.theme !== before.theme) {
    if (next.theme === "device") {
      changes.push({ key: settingKeys.appearanceMode, value: "device" });
    } else if (lightThemeIds.some((id) => id === next.theme)) {
      changes.push(
        { key: settingKeys.appearanceMode, value: "light" },
        { key: settingKeys.appearanceLightTheme, value: next.theme },
      );
    } else {
      changes.push(
        { key: settingKeys.appearanceMode, value: "dark" },
        { key: settingKeys.appearanceDarkTheme, value: next.theme },
      );
    }
  }
  if (
    next.light !== before.light &&
    !changes.some((c) => c.key === settingKeys.appearanceLightTheme)
  ) {
    changes.push({ key: settingKeys.appearanceLightTheme, value: next.light });
  }
  if (
    next.dark !== before.dark &&
    !changes.some((c) => c.key === settingKeys.appearanceDarkTheme)
  ) {
    changes.push({ key: settingKeys.appearanceDarkTheme, value: next.dark });
  }
  if (next.spacing !== before.spacing) {
    changes.push({ key: settingKeys.appearanceSpacing, value: next.spacing });
  }
  if (next.lessMotion !== before.lessMotion) {
    changes.push({ key: settingKeys.appearanceLessMotion, value: next.lessMotion });
  }
  if (next.largerText !== before.largerText) {
    changes.push({ key: settingKeys.appearanceLargerText, value: next.largerText });
  }
  return changes;
}
