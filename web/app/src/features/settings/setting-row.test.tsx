import { t } from "@bugaboo/messages";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { type Setting } from "../../shared/my-settings.ts";
import { doNothing } from "../../shared/do-nothing.ts";
import { SettingRow } from "./SettingRow.tsx";

/** A setting that is typed in, locked by an administrator. */
const locked: Setting = {
  key: "security.session-idle-minutes",
  name_key: "setting.security.session-idle-minutes.name",
  help_key: "setting.security.session-idle-minutes.help",
  kind: "whole-number",
  format: "free",
  choices: [],
  levels: ["organization", "person"],
  lockable: true,
  default: 30,
  value: 30,
  source: { kind: "organization" },
  own: true,
  locked: true,
  bounds: { least: 5, most: 1440 },
};

describe("cfg_06 a locked setting that is typed in", () => {
  it("still says what it is for and where its value comes from", () => {
    const markup = renderToStaticMarkup(
      <SettingRow setting={locked} actions={{ onChange: doNothing, onReset: doNothing }} />,
    );
    expect(markup).toContain(t("setting.security.session-idle-minutes.help"));
    expect(markup.replaceAll("&#x27;", "'")).toContain(t("setting.ui.source-organization"));
  });
});
