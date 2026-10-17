import { heartbeatSeconds, msPerSecond } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { signedInAdmin } from "./support/access.ts";
import { expect, test } from "./support/fixtures.ts";
import { anotherBrowser, signIn } from "./support/flows.ts";
import { issueOf, setStatusAndPriority, versionOf } from "./support/issues.ts";
import { prepareWorkflows } from "./support/workflows.ts";
import { openSimpleToChange, removeStatus } from "./support/workflows-pages.ts";

// The lock on a workflow (ADR-0019, ADR-0031): when another window takes the editing over while
// the question of *Publish* is open, the question closes and the note says why.

test.use({ preparation: { run: prepareWorkflows } });

test("wf_02 a question that is open is closed when the lock is lost, and the note says why", async ({
  page,
  browser,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const issue = await issueOf(admin, "WEB-2");
  await setStatusAndPriority(admin, issue, await versionOf(admin, issue), {
    status: "in-progress",
    priority: "normal",
  });
  await admin.dispose();
  await signIn(page, ada);
  await openSimpleToChange(page);
  await removeStatus(page, "In progress");
  await page.getByRole("button", { name: t("workflows.publish"), exact: true }).click();
  const question = page.getByRole("alertdialog", { name: t("workflows.publish-title") });
  await expect(question).toBeVisible();
  const other = await anotherBrowser(browser, server);
  await signIn(other, ada);
  await openSimpleToChange(other);
  // The window learns that the lock is gone with its next heartbeat: two of them are enough.
  const heartbeats = { timeout: heartbeatSeconds * 2 * msPerSecond };
  await expect(question).toHaveCount(0, heartbeats);
  await expect(page.getByText(t("workflows.lost"))).toBeVisible();
});
