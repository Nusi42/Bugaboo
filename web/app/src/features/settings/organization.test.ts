import { describe, expect, it } from "vitest";

import { type MySettings, type Setting } from "../../shared/my-settings.ts";
import { after } from "./organization.ts";

/** A setting of the organization as the server answers it, with only what these tests look at. */
function setting(key: string, value: Setting["value"], own: boolean, locked = false): Setting {
  return {
    key,
    name_key: "app.name",
    help_key: "app.name",
    kind: "text",
    format: "free",
    choices: [],
    levels: ["organization", "person"],
    lockable: true,
    default: "none",
    value,
    source: own ? { kind: "organization" } : { kind: "default" },
    own,
    locked,
  };
}

const list: MySettings = {
  settings: [setting("a.one", "dark", true, true), setting("a.two", "none", false)],
};

describe("cfg_06 an edit shows before the server has answered", () => {
  it("makes a changed value the organization's own", () => {
    const changed = after(list, { kind: "change", key: "a.two", value: "short" });
    expect(changed?.settings[1]).toMatchObject({
      value: "short",
      own: true,
      source: { kind: "organization" },
    });
    expect(changed?.settings[0]).toEqual(list.settings[0]);
  });

  it("takes the value away on a reset and keeps the lock", () => {
    const reset = after(list, { kind: "reset", key: "a.one" });
    expect(reset?.settings[0]).toMatchObject({
      value: "none",
      own: false,
      locked: true,
      source: { kind: "organization" },
    });
  });

  it("goes back to the default on a reset of a value that was not locked", () => {
    const reset = after(list, { kind: "reset", key: "a.two" });
    expect(reset?.settings[1]).toMatchObject({ own: false, source: { kind: "default" } });
  });

  it("shows an unlocked lock without a value of its own as the default again", () => {
    const reset = after(list, { kind: "reset", key: "a.one" });
    const unlocked = after(reset, { kind: "lock", key: "a.one", locked: false });
    expect(unlocked?.settings[0]).toMatchObject({
      locked: false,
      own: false,
      source: { kind: "default" },
    });
  });

  it("locks and unlocks only the named setting", () => {
    const unlocked = after(list, { kind: "lock", key: "a.one", locked: false });
    expect(unlocked?.settings[0]?.locked).toBe(false);
    const locked = after(list, { kind: "lock", key: "a.two", locked: true });
    expect(locked?.settings.map((one) => one.locked)).toEqual([true, true]);
  });

  it("changes nothing when there is nothing on screen yet", () => {
    expect(after(undefined, { kind: "reset", key: "a.one" })).toBeUndefined();
  });
});
