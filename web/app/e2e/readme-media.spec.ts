import { join } from "node:path";

import { t } from "@bugaboo/messages";

import { test } from "./support/fixtures.ts";
import { newIssueDialog, openNewIssueDialog } from "./support/issues.ts";
import {
  administratorPage,
  expectHeading,
  expectRows,
  folder,
  ISSUE,
  mediaFolder,
  recordingOf,
} from "./support/readme-media.ts";
import { shootLooksInto } from "./support/screenshots.ts";
import { PROJECT } from "./support/showcase-sample.ts";
import { titleBox } from "./support/windows.ts";
import { issueAddress, paths, projectIssuesAddress } from "../src/shared/paths.ts";

// The pictures and the recordings of the front page (README.md), made on the sample organization
// (`cargo xtask dataset sample` first; people with made-up names only). `npm run readme-media`
// runs this file with the folder `docs/media`; by hand:
//   BB_README_MEDIA_DIR=docs/media npx playwright test readme-media.spec --project=desktop
// The pictures are the window as a person sees it, in a light and a dark look; the recordings are
// GIFs with a drawn pointer, taken by `support/recording.ts`.

test.skip(mediaFolder === undefined, "Set BB_README_MEDIA_DIR to make the pictures.");
test.use({ sample: true });
test.describe.configure({ mode: "serial" });

test("pictures of the main screens, in both looks", async ({ browser, server }) => {
  const page = await administratorPage(browser, server, "computer");
  await page.goto(projectIssuesAddress(PROJECT));
  await expectRows(page);
  await shootLooksInto(page, folder(), "issue-list", "window");
  await page.goto(issueAddress(ISSUE));
  await expectHeading(page);
  await shootLooksInto(page, folder(), "issue-page", "window");
  await page.goto(paths.roles);
  await expectHeading(page);
  await shootLooksInto(page, folder(), "roles", "window");
  await page.goto(paths.workflows);
  await expectHeading(page);
  await shootLooksInto(page, folder(), "workflows", "window");
});

test("a picture of the list on a phone", async ({ browser, server }) => {
  const page = await administratorPage(browser, server, "phone");
  await page.goto(projectIssuesAddress(PROJECT));
  await expectRows(page);
  await shootLooksInto(page, folder(), "issue-list-phone", "window");
});

test("a recording: from the list to an issue", async ({ browser, server }) => {
  const { page, recording } = await recordingOf(browser, server);
  await page.goto(projectIssuesAddress(PROJECT));
  await expectRows(page);
  await recording.look();
  await recording.click(page.locator("[data-part='list-row'] a").first());
  await expectHeading(page);
  await recording.look(1500);
  recording.save(join(folder(), "open-an-issue.gif"));
});

test("a recording: finding an issue with quick search", async ({ browser, server }) => {
  const { page, recording } = await recordingOf(browser, server);
  await page.goto(paths.home);
  await expectHeading(page);
  await recording.look();
  await page.keyboard.press("Control+k");
  const box = page.getByRole("searchbox", { name: t("search.label"), exact: true });
  await recording.look(400);
  await recording.type(box, "login");
  const found = page.getByRole("option").first();
  await found.waitFor();
  await recording.look(1200);
  await recording.click(found);
  await expectHeading(page);
  await recording.look(1200);
  recording.save(join(folder(), "quick-search.gif"));
});

test("a recording: making an issue", async ({ browser, server }) => {
  const { page, recording } = await recordingOf(browser, server);
  await page.goto(projectIssuesAddress(PROJECT));
  await expectRows(page);
  await recording.look();
  await recording.pointAt(page.getByRole("button", { name: t("issues.new"), exact: true }).first());
  await openNewIssueDialog(page);
  await recording.look(500);
  await recording.type(titleBox(page), "The export button is hidden on small screens");
  await recording.look(600);
  await recording.click(
    newIssueDialog(page).getByRole("button", { name: t("issues.create"), exact: true }),
  );
  await newIssueDialog(page).waitFor({ state: "hidden" });
  await recording.look(1500);
  recording.save(join(folder(), "make-an-issue.gif"));
});
