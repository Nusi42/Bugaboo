import { settingDefaults, settingKeys } from "@bugaboo/api-client";
import { parseWindowSizes, type WindowSize, type WindowSizes } from "@bugaboo/ui";

import type { MySettings } from "./my-settings.ts";
import type { SettingChange } from "./setting-change.ts";
import { textOf } from "./setting-values.ts";

/** The sizes the person gave the windows of their forms, by the name of the form. */
export function sizesOf(settings?: MySettings): WindowSizes {
  return parseWindowSizes(textOf(settings, settingKeys.windowSizes, settingDefaults.windowSizes));
}

/** The size the person gave the window of one form, or nothing when it has its usual size. */
export function sizeOfForm(settings: MySettings | undefined, form: string): WindowSize | undefined {
  return sizesOf(settings)[form];
}

/** The text of the setting when the window of `form` ends up `size` big and `text` has the others. */
function withSize(text: string, form: string, size: WindowSize): string {
  return JSON.stringify({ ...parseWindowSizes(text), [form]: size });
}

/**
 * The change of the setting when the window of `form` ended up `size` big: the other forms keep
 * theirs, also those another tab set meanwhile (`rebase`, M5-F97). The server keeps whole pixels,
 * so the size is rounded.
 */
export function changeForSize(
  settings: MySettings | undefined,
  form: string,
  size: WindowSize,
): SettingChange {
  const rounded = {
    width: Math.round(size.width),
    height: Math.round(size.height),
  };
  return {
    key: settingKeys.windowSizes,
    value: withSize(
      textOf(settings, settingKeys.windowSizes, settingDefaults.windowSizes),
      form,
      rounded,
    ),
    rebase: (current) =>
      withSize(typeof current === "string" ? current : settingDefaults.windowSizes, form, rounded),
  };
}
