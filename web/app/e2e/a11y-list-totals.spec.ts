import { t } from "@bugaboo/messages";

import { listWord } from "../src/shared/list-columns.ts";
import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada, signInThroughApi } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { createTask } from "./support/dashboard-issues.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";

// The accessibility suite for the totals of a grouped list (TEST-A11Y, QRY-08): the list grouped by priority with its
// rows of totals, in the four themes on a computer and a phone.

test.use({ preparation: { run: prepareWeb } });

test("a11y a grouped list with its rows of totals", async ({ page, server }) => {
  const admin = await signInThroughApi(server, ada);
  await createTask(admin, "Fix the header", { priority: "high", estimated_time: 90 });
  await createTask(admin, "Write the guide", { priority: "low" });
  await admin.dispose();
  await signIn(page, ada);
  const columns = [listWord.status, listWord.id, listWord.title, listWord.estimatedTime];
  await page.goto(
    `${projectIssuesAddress("WEB")}?group=${listWord.priority}&cols=${encodeURIComponent(columns.join(","))}`,
  );
  await expect(
    page.getByRole("row", {
      name: t("issues.totals-of", { group: t("priority.high") }),
      exact: false,
    }),
  ).toBeVisible();
  await expectGoodInEveryLook(page);
});
