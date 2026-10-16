import { settingKeys } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import type { MySettings, Setting } from "./my-settings.ts";
import { changeForSize, sizeOfForm, sizesOf } from "./window-sizes.ts";

const form = { newIssue: "new-issue", other: "other-form" } as const;

/** The setting of the sizes as the server answers it, holding `text`. */
function settings(text: string): MySettings {
  const sizes: Setting = {
    key: settingKeys.windowSizes,
    name_key: "app.name",
    help_key: "app.name",
    kind: "text",
    format: "window_sizes",
    choices: [],
    levels: ["person"],
    lockable: false,
    default: "{}",
    value: text,
    source: { kind: "person" },
    own: true,
    locked: false,
  };
  return { settings: [sizes] };
}

describe("ux_6_3 the size of a window is remembered per person and form", () => {
  it("is nothing before the person resized a form", () => {
    expect(sizeOfForm(undefined, form.newIssue)).toBeUndefined();
    expect(sizeOfForm(settings("{}"), form.newIssue)).toBeUndefined();
  });

  it("reads the size of the form from the person's settings", () => {
    const own = settings(JSON.stringify({ [form.newIssue]: { width: 900, height: 640 } }));
    expect(sizeOfForm(own, form.newIssue)).toEqual({ width: 900, height: 640 });
    expect(sizeOfForm(own, form.other)).toBeUndefined();
    expect(sizesOf(settings("not json"))).toEqual({});
  });

  it("changes the one setting by the size of one form and keeps the others", () => {
    const own = settings(JSON.stringify({ [form.other]: { width: 700, height: 500 } }));
    const change = changeForSize(own, form.newIssue, { width: 900.4, height: 640.6 });
    expect(change.key).toBe(settingKeys.windowSizes);
    expect(JSON.parse(String(change.value))).toEqual({
      [form.other]: { width: 700, height: 500 },
      [form.newIssue]: { width: 900, height: 641 },
    });
  });
});
