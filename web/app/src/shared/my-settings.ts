import { settingKeys, type ReadonlyDeep, type Schema, type SettingKey } from "@bugaboo/api-client";
import {
  darkThemeIds,
  defaultAppearance,
  lightThemeIds,
  spacingChoices,
  type Appearance,
  type DarkThemeId,
  type LightThemeId,
  type SpacingChoice,
} from "@bugaboo/ui";

import { api } from "./api.ts";
import { queryClient } from "./query-client.ts";
import { deviceRegion, isKnownTimeZone, type Region } from "./region.ts";
import { parseOverrides, type ShortcutOverrides } from "./shortcuts.ts";

/** One setting of the person, with the value that counts for them and where it comes from. */
export type Setting = Schema<"SettingBody">;
/** The settings a person can choose for themselves (*My settings*). */
export type MySettings = Schema<"SettingsBody">;
/** What a setting holds: yes or no, a whole number, or text. */
export type SettingValue = Setting["value"];

/**
 * The settings of the person who is signed in. The answer is never kept by the browser (the server
 * says so) and the cache forgets it when the person signs out, so the next person at this device
 * finds nothing of it (M2-F13).
 */
export function mySettingsOptions() {
  return api.query.queryOptions("get", "/api/v1/me/settings", undefined, { staleTime: 0 });
}

/**
 * Makes sure the app has the person's settings before the first page is drawn, so the look is the
 * person's from the first frame. When the server cannot answer, the person gets the standard
 * settings and everything else still works.
 */
export async function loadMySettings(): Promise<void> {
  try {
    await queryClient.query({ ...mySettingsOptions(), staleTime: "static" });
  } catch {
    // The pages ask again, and a lost sign-in is told to the person by the cache itself.
  }
}

/** The page that opens after signing in: the person's start page, or the home page. */
export async function loadStartPage(): Promise<string> {
  try {
    return startPageOf(await queryClient.query(mySettingsOptions()));
  } catch {
    return "/";
  }
}

/** The key of the query in the cache of the app. */
export function mySettingsKey() {
  return mySettingsOptions().queryKey;
}

/** The settings of the person, if the app has them (it asks when the person is signed in). */
export function currentMySettings(): MySettings | undefined {
  const known: unknown = queryClient.getQueryData(mySettingsKey());
  return isMySettings(known) ? known : undefined;
}

/** Whether what the cache holds under the key is the answer about the person's settings. */
export function isMySettings(value: unknown): value is MySettings {
  return typeof value === "object" && value !== null && "settings" in value;
}

/** Whether a key the server named is one this app knows. */
export function isSettingKey(key: string): key is SettingKey {
  return Object.values<string>(settingKeys).includes(key);
}

/** One setting by its key. */
export function settingOf(settings: MySettings | undefined, key: SettingKey): Setting | undefined {
  return settings?.settings.find((setting) => setting.key === key);
}

function textOf(settings: MySettings | undefined, key: SettingKey, fallback: string): string {
  const value = settingOf(settings, key)?.value;
  return typeof value === "string" ? value : fallback;
}

function flagOf(settings: MySettings | undefined, key: SettingKey, fallback: boolean): boolean {
  const value = settingOf(settings, key)?.value;
  return typeof value === "boolean" ? value : fallback;
}

/** The theme choice: follow the device, or always light, or always dark. */
export type Mode = "device" | "light" | "dark";
/** Where an issue opens. */
export type IssueView = "focus" | "panel";

/** What the person chose about how Bugaboo looks, as the settings hold it. */
export interface PersonalAppearance {
  readonly mode: Mode;
  readonly light: LightThemeId;
  readonly dark: DarkThemeId;
  readonly spacing: SpacingChoice;
  readonly lessMotionOn: boolean;
  readonly largerTextOn: boolean;
  readonly issueView: IssueView;
}

function oneOf<Value extends string>(
  list: readonly Value[],
  value: string,
  fallback: Value,
): Value {
  return list.find((item) => item === value) ?? fallback;
}

const modes: readonly Mode[] = ["device", "light", "dark"];
const issueViews: readonly IssueView[] = ["focus", "panel"];

/** The look the settings describe; a value that is not a valid choice is the standard one. */
export function appearanceOf(settings?: MySettings): PersonalAppearance {
  return {
    mode: oneOf(modes, textOf(settings, settingKeys.appearanceMode, "device"), "device"),
    light: oneOf(
      lightThemeIds,
      textOf(settings, settingKeys.appearanceLightTheme, defaultAppearance.light),
      defaultAppearance.light,
    ),
    dark: oneOf(
      darkThemeIds,
      textOf(settings, settingKeys.appearanceDarkTheme, defaultAppearance.dark),
      defaultAppearance.dark,
    ),
    spacing: oneOf(
      spacingChoices,
      textOf(settings, settingKeys.appearanceSpacing, defaultAppearance.spacing),
      defaultAppearance.spacing,
    ),
    lessMotionOn: flagOf(settings, settingKeys.appearanceLessMotion, false),
    largerTextOn: flagOf(settings, settingKeys.appearanceLargerText, false),
    issueView: oneOf(
      issueViews,
      textOf(settings, settingKeys.appearanceIssueView, "focus"),
      "focus",
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

/** The region the settings describe. */
export function regionOf(settings?: MySettings): Region {
  const text = (key: SettingKey, fallback: string): string => textOf(settings, key, fallback);
  const zone = text(settingKeys.regionTimeZone, deviceRegion.timeZone);
  return {
    timeZone:
      zone === deviceRegion.timeZone || isKnownTimeZone(zone) ? zone : deviceRegion.timeZone,
    dateFormat: oneOf(
      ["device", "day-month-year", "month-day-year", "year-month-day"],
      text(settingKeys.regionDateFormat, "device"),
      "device",
    ),
    numberFormat: oneOf(
      ["device", "point", "comma", "space"],
      text(settingKeys.regionNumberFormat, "device"),
      "device",
    ),
    firstDay: oneOf(
      ["device", "monday", "sunday", "saturday"],
      text(settingKeys.regionFirstDay, "device"),
      "device",
    ),
  };
}

/** The address of the page that opens after signing in. */
export function startPageOf(settings?: MySettings): string {
  return textOf(settings, settingKeys.startPage, "/");
}

/** The keys the person chose for shortcuts. */
export function shortcutKeysOf(settings?: MySettings): ShortcutOverrides {
  return parseOverrides(textOf(settings, settingKeys.shortcutsKeys, "{}"));
}

/** One change of a person's own settings: a new value, or `null` to take it away. */
export interface SettingChange {
  readonly key: SettingKey;
  readonly value: SettingValue | null;
}

/** The settings after a change, before the server has answered: the value, as the person's own. */
export function withChange(before: MySettings, change: SettingChange): MySettings {
  return {
    settings: before.settings.map((setting): ReadonlyDeep<Setting> => {
      if (setting.key !== change.key) {
        return setting;
      }
      if (change.value === null) {
        // The value from above is not known here; the answer of the server brings it.
        return { ...setting, value: setting.default, own: false, source: { kind: "default" } };
      }
      return { ...setting, value: change.value, own: true, source: { kind: "person" } };
    }),
  };
}
