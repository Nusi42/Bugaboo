import { type Page } from "@playwright/test";

import { paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { chooseTheme, themes, type Theme } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareLook } from "./support/list-look.ts";
import { screenshotFolder, shotWholeAs } from "./support/screenshots.ts";
import { inOrder } from "./support/sequence.ts";

// The screens that list their rows with `ListEditor`, in the four themes, on a computer and on a
// phone: the rows end where the row ends (a mark or a count at the end edge), and the project list
// has its own columns. For the maintainer's review of M4. Only taken when `BB_SCREENSHOT_DIR` names the
// folder they go to:
//   BB_SCREENSHOT_DIR=... npx playwright test screenshots-lists.spec --project=desktop --project=phone
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareLook } });

const MILESTONE = "M4/lists";

/** The screens, by the address they open at and the name of their pictures. */
const screens = [
  { name: "groups", address: paths.groups },
  { name: "relation-kinds", address: paths.relationKinds },
  { name: "labels-and-tags", address: paths.labels },
  { name: "issue-types", address: paths.issueTypes },
  { name: "projects", address: paths.projects },
  { name: "shortcuts", address: paths.shortcuts },
] as const;

/** Opens a screen and takes it in the theme. */
async function take(page: Page, theme: Theme, address: string, name: string, layout: string) {
  await page.goto(address);
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await chooseTheme(page, theme);
  await shotWholeAs(page, `${name}-${layout}`, MILESTONE, theme.id);
}

for (const theme of themes) {
  test(`the lists in ${theme.id}`, async ({ page }, info) => {
    test.setTimeout(180_000);
    await signIn(page, ada);
    await inOrder(
      screens.map(
        (screen) => () => take(page, theme, screen.address, screen.name, info.project.name),
      ),
    );
  });
}
