import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { partIn } from "./support/parts.ts";

// An organization that changes no layout sees the issue page as before the lengths of time were
// fields of the layout (TIME-03, CFG-01): the three stand together at the end of the fields, in
// the order *Estimated time*, *Time left*, *Time spent*.

test.use({ preparation: { run: prepareWeb } });

test("time_03 the default layout of the issue page shows the lengths of time last, in their usual order", async ({
  page,
}) => {
  await signIn(page, ada);
  await openIssueWithDetails(page, "WEB-1");
  const rows = partIn(page, "name-value.name");
  // The rows of Followers and of *Time spent* come from reads of their own, which end last.
  await expect(rows.filter({ hasText: t("followers.name") })).toBeVisible();
  await expect(rows.filter({ hasText: t("issue-field.time-spent") })).toBeVisible();
  const shown = await rows.allInnerTexts();
  const estimated = t("issue-field.estimated-time");
  const lengths = [estimated, t("issue-field.time-left"), t("issue-field.time-spent")];
  expect(shown.filter((name) => lengths.includes(name))).toEqual(lengths);
  // They follow the fields the starting layout places before them.
  const first = shown.indexOf(estimated);
  expect(shown.indexOf(t("issue-field.assigned-to"))).toBeLessThan(first);
  expect(shown.indexOf(t("issue-field.priority"))).toBeLessThan(first);
  expect(shown.indexOf(t("issue-field.reported-by"))).toBeLessThan(first);
  expect(shown.indexOf(t("labels.labels-heading"))).toBeLessThan(first);
});
