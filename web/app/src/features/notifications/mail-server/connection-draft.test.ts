import { settingKeys } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { type MySettings } from "../../../shared/my-settings.ts";
import { changesOf, draftOf } from "./connection-draft.ts";

/** A setting as the server lists it, with the value it holds. */
function setting(key: string, value: string | number): MySettings["settings"][number] {
  return {
    key,
    name_key: "setting.mail.host.name",
    help_key: "setting.mail.host.help",
    kind: typeof value === "number" ? "integer" : "text",
    format: "free",
    choices: [],
    levels: ["organization"],
    lockable: false,
    default: value,
    value,
    source: { kind: "default" },
    own: false,
    locked: false,
    bounds: { least: 1, most: 65535 },
  };
}

const settings: MySettings = {
  settings: [
    setting(settingKeys.mailHost, "mail.example.test"),
    setting(settingKeys.mailPort, 587),
    setting(settingKeys.mailSecurity, "starttls"),
    setting(settingKeys.mailUsername, ""),
    setting(settingKeys.mailFrom, "bugaboo@example.test"),
  ],
};

describe("ntf_02 the form of the mail server starts from the settings", () => {
  it("shows what the settings hold now, and no password", () => {
    expect(draftOf(settings)).toEqual({
      host: "mail.example.test",
      port: "587",
      security: "starttls",
      username: "",
      from: "bugaboo@example.test",
      password: "",
    });
  });
});

describe("ntf_02 only what the person changed is saved", () => {
  it("lists the settings whose text differs, with the spaces around it cut off", () => {
    const draft = { ...draftOf(settings), host: "  smtp.example.test ", port: "465" };
    const found = changesOf(settings, draft);
    expect(found.problem).toBeUndefined();
    expect(found.changes).toEqual([
      [settingKeys.mailHost, "smtp.example.test"],
      [settingKeys.mailPort, 465],
    ]);
  });

  it("lists nothing when nothing changed", () => {
    expect(changesOf(settings, draftOf(settings)).changes).toEqual([]);
  });

  it("refuses a port that is no whole number inside the bounds, and says between which", () => {
    for (const port of ["", "many", "0", "70000", "58.7"]) {
      const found = changesOf(settings, { ...draftOf(settings), port });
      expect(found.changes).toEqual([]);
      expect(found.problem).toBeDefined();
    }
  });
});
