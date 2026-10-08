import { settingKeys, type SettingKey } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { appearanceForShell, appearanceOf, changesForAppearance } from "./appearance-setting.ts";
import {
  isSettingKey,
  settingOf,
  type MySettings,
  type Setting,
  type SettingValue,
} from "./my-settings.ts";
import { regionOf } from "./region.ts";
import { withChange } from "./setting-change.ts";
import { shortcutKeysOf } from "./shortcuts-source.ts";
import { startPageOf } from "./start-page-setting.ts";

/** A setting as the server answers it, with only what these tests look at. */
function setting(key: SettingKey, value: SettingValue, own = true): Setting {
  return {
    key,
    name_key: "app.name",
    help_key: "app.name",
    kind: "text",
    format: "free",
    choices: [],
    levels: ["organization", "person"],
    lockable: false,
    default: "",
    value,
    source: own ? { kind: "person" } : { kind: "default" },
    own,
    locked: false,
  };
}

function list(values: Readonly<Partial<Record<SettingKey, SettingValue>>>): MySettings {
  const settings: Setting[] = [];
  for (const [key, value] of Object.entries(values)) {
    if (isSettingKey(key) && value !== undefined) {
      settings.push(setting(key, value));
    }
  }
  return { settings };
}

/** The time zone the region uses when the person chose `zone`. */
function zoneOf(zone: string): string {
  return regionOf(list({ [settingKeys.regionTimeZone]: zone })).timeZone;
}

/** The look the shell applies for a mode, with a light theme, a dark theme and less motion chosen. */
function shellLook(mode: string) {
  return appearanceForShell(
    appearanceOf(
      list({
        [settingKeys.appearanceMode]: mode,
        [settingKeys.appearanceLightTheme]: "paper",
        [settingKeys.appearanceDarkTheme]: "midnight",
        [settingKeys.appearanceLessMotion]: true,
      }),
    ),
  );
}

describe("cfg_04 the look a person chose, as the server holds it", () => {
  it("is the standard look when nothing is set, and when a value is not a choice", () => {
    expect(appearanceOf().mode).toBe("device");
    const look = appearanceOf(
      list({
        [settingKeys.appearanceMode]: "sometimes",
        [settingKeys.appearanceLightTheme]: "midnight",
        [settingKeys.appearanceSpacing]: 12,
        [settingKeys.appearanceLessMotion]: "yes",
      }),
    );
    expect(look).toEqual({
      mode: "device",
      light: "daylight",
      dark: "dusk",
      spacing: "comfortable",
      lessMotionOn: false,
      largerTextOn: false,
      issueView: "focus",
    });
  });

  it("is applied by the shell as one theme, or Match my device", () => {
    expect(shellLook("device")).toMatchObject({
      theme: "device",
      light: "paper",
      dark: "midnight",
    });
    expect(shellLook("light").theme).toBe("paper");
    expect(shellLook("dark").theme).toBe("midnight");
    expect(shellLook("dark").lessMotion).toBe(true);
  });
});

describe("cfg_04 a change of the look is sent as the settings that differ", () => {
  it("changes only the settings that differ, and a theme picked on the page sets the mode", () => {
    const before = appearanceForShell(appearanceOf());
    expect(changesForAppearance(before, before)).toEqual([]);
    expect(
      changesForAppearance(before, { ...before, largerText: true, spacing: "compact" }),
    ).toEqual([
      { key: settingKeys.appearanceSpacing, value: "compact" },
      { key: settingKeys.appearanceLargerText, value: true },
    ]);
    expect(changesForAppearance(before, { ...before, theme: "paper" })).toEqual([
      { key: settingKeys.appearanceMode, value: "light" },
      { key: settingKeys.appearanceLightTheme, value: "paper" },
    ]);
    expect(changesForAppearance(before, { ...before, theme: "midnight" })).toEqual([
      { key: settingKeys.appearanceMode, value: "dark" },
      { key: settingKeys.appearanceDarkTheme, value: "midnight" },
    ]);
    expect(changesForAppearance(before, { ...before, theme: "device" })).toEqual([]);
    expect(changesForAppearance(before, { ...before, light: "paper", dark: "midnight" })).toEqual([
      { key: settingKeys.appearanceLightTheme, value: "paper" },
      { key: settingKeys.appearanceDarkTheme, value: "midnight" },
    ]);
  });
});

describe("cfg_04 region, start page and shortcut keys", () => {
  it("reads the region, and the device where nothing is set", () => {
    expect(regionOf()).toEqual({
      timeZone: "device",
      dateFormat: "device",
      numberFormat: "device",
      firstDay: "device",
    });
    expect(
      regionOf(
        list({
          [settingKeys.regionTimeZone]: "Europe/Berlin",
          [settingKeys.regionDateFormat]: "day-month-year",
          [settingKeys.regionNumberFormat]: "nonsense",
          [settingKeys.regionFirstDay]: "monday",
        }),
      ),
    ).toEqual({
      timeZone: "Europe/Berlin",
      dateFormat: "day-month-year",
      numberFormat: "device",
      firstDay: "monday",
    });
  });

  it("shows the times in the device's zone when the browser does not know the one that was chosen", () => {
    expect(zoneOf("America/New_York")).toBe("America/New_York");
    expect(zoneOf("Mars/Olympus")).toBe("device");
  });

  it("opens the home page when no start page is set", () => {
    expect(startPageOf()).toBe("/");
    expect(startPageOf(list({ [settingKeys.startPage]: "/me/region" }))).toBe("/me/region");
  });

  it("reads the keys a person chose and leaves out what is not a valid choice", () => {
    expect(shortcutKeysOf()).toEqual({});
    expect(
      shortcutKeysOf(list({ [settingKeys.shortcutsKeys]: '{"go-home":["g i"],"nonsense":["x"]}' })),
    ).toEqual({ "go-home": ["g i"] });
    expect(shortcutKeysOf(list({ [settingKeys.shortcutsKeys]: "not json" }))).toEqual({});
  });

  it("knows the keys of the registry and no others", () => {
    expect(isSettingKey("region.time-zone")).toBe(true);
    expect(isSettingKey("region.nothing")).toBe(false);
  });
});

describe("cfg_04 the start page is only ever a page of this app", () => {
  it("opens the home page for an address that is not one", () => {
    for (const bad of ["//evil.test", "/\\evil.test", "https://evil.test/", "me/region", ""]) {
      expect(startPageOf(list({ [settingKeys.startPage]: bad }))).toBe("/");
    }
  });
});

describe("cfg_04 a change shows before the server has answered", () => {
  it("makes the value the person's own, and a reset the standard value", () => {
    const before = list({ [settingKeys.appearanceMode]: "dark" });
    const changed = withChange(before, { key: settingKeys.appearanceMode, value: "light" });
    expect(settingOf(changed, settingKeys.appearanceMode)).toMatchObject({
      value: "light",
      own: true,
      source: { kind: "person" },
    });
    const reset = withChange(changed, { key: settingKeys.appearanceMode, value: null });
    expect(settingOf(reset, settingKeys.appearanceMode)).toMatchObject({
      value: "",
      own: false,
      source: { kind: "default" },
    });
    // The list it came from is left as it was.
    expect(settingOf(before, settingKeys.appearanceMode)?.value).toBe("dark");
  });
});
