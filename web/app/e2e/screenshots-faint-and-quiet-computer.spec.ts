import { t } from "@bugaboo/messages";

import { bob } from "./support/accounts.ts";
import { chooseTheme } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { screenshotFolder, shotWholeAs } from "./support/screenshots.ts";
import {
  changeColor,
  colorField,
  deleteThemesOf,
  openEditor,
  prepareThemes,
  readabilityLine,
  uploadedTheme,
} from "./support/theme-editor.ts";

// The pictures for the maintainer's choice about quiet and hint text (M8-F40): in Paper and in
// Daylight the two are about 1.1:1 apart. Each theme as it is, and with one proposed change, as
// the theme editor rates it, in `M8/f40` below BB_SCREENSHOT_DIR. The built-in themes stay as
// they are: the proposals are personal themes.

test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareThemes } });

const FOLDER = "M8/f40";

/**
 * The proposals: a darker quiet text, so it stands apart from the hint text (about 1.4:1 instead of
 * 1.1:1) and still reads easily on every background of the theme.
 */
const proposals = [
  { theme: "paper", label: t("theme.paper"), quiet: "#463f37" },
  { theme: "daylight", label: t("theme.daylight"), quiet: "#464b5a" },
] as const;

test.beforeEach(async ({ server }) => {
  await deleteThemesOf(server, bob);
});

for (const proposal of proposals) {
  test(`screens: quiet and hint text in ${proposal.theme}, as now and proposed`, async ({
    page,
    server,
  }) => {
    const theme = await uploadedTheme(server, bob, "personal", {
      name: `${proposal.theme} quiet text`,
      extends: proposal.theme,
      values: {},
    });
    await signIn(page, bob);
    await openEditor(page, theme);
    const text = t("theme-editor.group.text");
    await changeColor(page, text, t("theme-color.fg-muted"));
    await changeColor(page, text, t("theme-color.fg-subtle"));
    await chooseTheme(page, { id: proposal.theme, label: proposal.label });
    await shotWholeAs(page, "quiet-and-hint-now", FOLDER, proposal.theme);
    await colorField(page, t("theme-color.fg-muted")).fill(proposal.quiet);
    await expect(readabilityLine(page)).not.toContainText(
      t("theme-editor.readability-fix", { count: 1 }),
    );
    await shotWholeAs(page, "quiet-and-hint-proposed", FOLDER, proposal.theme);
  });
}
