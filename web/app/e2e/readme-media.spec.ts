import { join } from "node:path";

import { test } from "./support/fixtures.ts";
import { themeCycle } from "./support/readme-hero.ts";
import {
  administratorPage,
  expectHeading,
  expectRows,
  folder,
  mediaFolder,
  recordingOf,
  SHARP,
} from "./support/readme-media.ts";
import {
  createReadmeOrganization,
  readmeIssue,
  readmeProject,
  readmeRepairs,
} from "./support/readme-organization.ts";
import { shootLooksInto } from "./support/screenshots.ts";
import {
  findAnIssue,
  openAnIssue,
  reportAnIssue,
  showDrawing,
} from "./support/readme-recordings.ts";
import {
  addressInProject,
  issueAddress,
  paths,
  projectIssuesAddress,
} from "../src/shared/paths.ts";

// The pictures and the recordings of the front page (README.md), made on Quillmoor Cycle Works, a
// made-up bicycle workshop (`support/readme-organization-data.ts`) that the preparation sets up on
// an empty server. `npm run readme-media` runs this file with the folder `docs/media`; by hand:
//   BB_README_MEDIA_DIR=docs/media npx playwright test readme-media.spec --project=desktop
// The front picture cycles through the four themes; the other pictures are the window in a light
// and a dark look; the recordings are GIFs with a drawn pointer, taken by `support/recording.ts`.

test.skip(mediaFolder === undefined, "Set BB_README_MEDIA_DIR to make the pictures.");
test.use({ preparation: { run: createReadmeOrganization } });
test.describe.configure({ mode: "serial" });

test("the front picture: the list of the workshop in the four themes", async ({
  browser,
  server,
}) => {
  const page = await administratorPage(browser, server, "computer", SHARP.front);
  await page.goto(projectIssuesAddress(readmeProject));
  await expectRows(page);
  await themeCycle(page, "issue-list-themes");
});

test("pictures of the main screens, in both looks", async ({ browser, server }) => {
  const page = await administratorPage(browser, server, "computer");
  await page.goto(issueAddress(readmeIssue));
  await expectHeading(page);
  await shootLooksInto(page, folder(), "issue-page", "window");
  await page.goto(addressInProject(paths.projectWorkflows, readmeRepairs));
  await expectHeading(page);
  await showDrawing(page);
  await shootLooksInto(page, folder(), "workflows", "window");
  await page.goto(paths.roles);
  await expectHeading(page);
  await shootLooksInto(page, folder(), "roles", "window");
});

test("a picture of the list on a phone", async ({ browser, server }) => {
  const page = await administratorPage(browser, server, "phone");
  await page.goto(projectIssuesAddress(readmeRepairs));
  await expectRows(page);
  await shootLooksInto(page, folder(), "issue-list-phone", "window");
});

test("a recording: from the list to an issue", async ({ browser, server }) => {
  const { page, recording } = await recordingOf(browser, server);
  await openAnIssue(page, recording);
  recording.save(join(folder(), "open-an-issue.gif"));
});

test("a recording: finding an issue with quick search", async ({ browser, server }) => {
  const { page, recording } = await recordingOf(browser, server);
  await findAnIssue(page, recording);
  recording.save(join(folder(), "quick-search.gif"));
});

test("a recording: reporting an issue", async ({ browser, server }) => {
  const { page, recording } = await recordingOf(browser, server);
  await reportAnIssue(page, recording);
  recording.save(join(folder(), "report-an-issue.gif"));
});
