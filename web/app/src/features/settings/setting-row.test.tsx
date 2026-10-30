import { t } from "@bugaboo/messages";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { type Setting } from "../../shared/my-settings.ts";
import { doNothing } from "../../shared/do-nothing.ts";
import { SettingRow } from "./SettingRow.tsx";
import { SettingsForm } from "./SettingsForm.tsx";

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

/** A setting with a switch: it is saved as soon as it changes. */
const switched: Setting = {
  ...locked,
  key: "security.two-step-required",
  name_key: "setting.security.two-step-required.name",
  help_key: "setting.security.two-step-required.help",
  kind: "yes-no",
  format: "free",
  default: false,
  value: false,
  locked: false,
  bounds: null,
};

/** Nothing is being saved: a word of the save state, not text for people. */
const idle = "idle";

/** The form of a page with these settings, as markup. */
function formOf(settings: readonly Setting[]): string {
  return renderToStaticMarkup(
    <SettingsForm
      settings={settings}
      actions={{ onChange: doNothing, onReset: doNothing }}
      saveState={idle}
      problem={null}
      searchable={false}
    />,
  ).replaceAll("&#x27;", "'");
}

describe("cfg_06 a page whose settings save in two ways", () => {
  it("says which save at once and which wait for Save, where a switch stands beside a box", () => {
    const typed = { ...locked, locked: false };
    expect(formOf([typed, switched])).toContain(t("setting.ui.saved-at-once"));
    expect(formOf([typed])).not.toContain(t("setting.ui.saved-at-once"));
    expect(formOf([switched])).not.toContain(t("setting.ui.saved-at-once"));
  });
});
