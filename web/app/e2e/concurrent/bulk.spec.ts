import { signedInAdmin } from "../support/access.ts";
import {
  atVersion,
  bulkBody,
  bulkRoute,
  changeMany,
  findBulkRoute,
  statusOf,
} from "../support/bulk.ts";
import { signedInPage } from "../support/crowd-pages.ts";
import { ben, crowdIssues, prepareCrowd } from "../support/crowd.ts";
import { descriptionBox, startEditing, typeUnderLock } from "../support/description.ts";
import { expect, test } from "../support/fixtures.ts";

// Changing many issues while somebody edits one of them (architecture §11, M3 WP-10, ISS-09).
// The route that changes many issues comes with WP-08. Until the server has it, the scenario is
// skipped and says why. TEST-CONCURRENT fails once the migration of WP-08 (`m0011`) is in the code and
// the skip is still here: then `bulkBody` and the search for the route in support/bulk.ts are
// adapted to the real route and the skip is taken out.

test.use({ preparation: { run: prepareCrowd } });

test("iss_09_changing_many_issues_skips_the_one_somebody_is_editing_and_names_it", async ({
  browser,
  server,
  viewport,
  hasTouch,
}) => {
  const admin = await signedInAdmin(server);
  test.skip(
    (await findBulkRoute(admin)) === undefined,
    "The route that changes many issues (WP-08) is not in this build yet.",
  );
  const editor = await signedInPage(browser, server, ben, { viewport, hasTouch });
  await startEditing(editor, crowdIssues.bulk[0]);
  await typeUnderLock(editor, "Ben is still writing this");
  const edited = await atVersion(admin, crowdIssues.bulk[0]);
  const others = await Promise.all(crowdIssues.bulk.slice(1).map((key) => atVersion(admin, key)));
  const answer = await changeMany(
    admin,
    await bulkRoute(admin),
    bulkBody([edited, ...others], "in-progress"),
  );
  expect(answer.status).toBeLessThan(300);
  // The one under edit is named and left alone; the others changed.
  expect(JSON.stringify(answer.body)).toContain(edited.key);
  expect(await statusOf(admin, edited)).toBe("open");
  expect(await Promise.all(others.map((other) => statusOf(admin, other)))).toEqual(
    others.map(() => "in-progress"),
  );
  // Ben's text is still his and can be saved.
  await expect(descriptionBox(editor)).toHaveValue("Ben is still writing this");
  await admin.dispose();
});
