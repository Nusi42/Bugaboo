import type { SettingKey } from "@bugaboo/api-client";

import { settingOf, type MySettings } from "./my-settings.ts";

/** The text a setting holds, or `fallback` when the settings have none or it is not text. */
export function textOf(
  settings: MySettings | undefined,
  key: SettingKey,
  fallback: string,
): string {
  const value = settingOf(settings, key)?.value;
  return typeof value === "string" ? value : fallback;
}

/** The yes or no a setting holds, or `fallback` when the settings have none or it is not one. */
export function flagOf(
  settings: MySettings | undefined,
  key: SettingKey,
  fallback: boolean,
): boolean {
  const value = settingOf(settings, key)?.value;
  return typeof value === "boolean" ? value : fallback;
}

/** `value` if it is one of the choices, else `fallback`: a value that is no longer a choice never breaks a page. */
export function oneOf<Value extends string>(
  list: readonly Value[],
  value: string,
  fallback: Value,
): Value {
  return list.find((item) => item === value) ?? fallback;
}
