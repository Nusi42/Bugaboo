import { test } from "./support/fixtures.ts";
import { themes } from "./support/checks.ts";
import {
  takeCheckAccess,
  takeMove,
  takeNewIssue,
  takeOfferedIn,
  takeProjectLabels,
  takeProjectTypes,
} from "./support/offers-look.ts";
import { prepareOffersLook } from "./support/offers-state.ts";
import { screenshotFolder } from "./support/screenshots.ts";

// The screens of M4 WP-11 for the maintainer's review, in the four themes, on a computer and on a phone:
// the issue types and the labels of a project, the panel *Offered in*, New issue with the types a
// project offers, the note on a move, and *Check access* with its question. Only taken when
// `BB_SCREENSHOT_DIR` names the folder they go to:
//   BB_SCREENSHOT_DIR=... npx playwright test screenshots-offers.spec --project=desktop --project=phone
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareOffersLook } });
test.describe.configure({ mode: "serial" });

for (const theme of themes) {
  test.describe(`offers in ${theme.id}`, () => {
    test("the issue types of a project", ({ page }, info) =>
      takeProjectTypes(page, theme, info.project.name));
    test("the labels of a project", ({ page }, info) =>
      takeProjectLabels(page, theme, info.project.name));
    test("the panel Offered in", ({ page }, info) => takeOfferedIn(page, theme, info.project.name));
    test("New issue with the types the project offers", ({ page }, info) =>
      takeNewIssue(page, theme, info.project.name));
    test("the note on a move", ({ page }, info) => takeMove(page, theme, info.project.name));
    test("Check access asks for the issue type", ({ page }, info) =>
      takeCheckAccess(page, theme, info.project.name));
  });
}
