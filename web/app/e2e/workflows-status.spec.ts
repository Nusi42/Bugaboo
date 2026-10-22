import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { issueAddress } from "../src/shared/paths.ts";
import { ada, bob } from "./support/accounts.ts";
import { roleId, roleName, signedInAdmin } from "./support/access.ts";
import { expect, test } from "./support/fixtures.ts";
import { anotherBrowser, signIn } from "./support/flows.ts";
import { expectStatus, statusButton } from "./support/issues.ts";
import {
  changeSimple,
  createBug,
  mustFill,
  onlyForRoles,
  prepareWorkflows,
} from "./support/workflows.ts";
import { serverWord } from "./support/server-words.ts";

// The status button and its list (WF-04, M4 WP-07, UX §4.4): the list holds the status changes the
// person may make now, in the words of their buttons.

test.use({ preparation: { run: prepareWorkflows } });

/** Opens the list of the big status button. */
async function openMenu(page: Page, key: string) {
  await page.goto(issueAddress(key));
  await expect(statusButton(page)).toBeVisible();
  await statusButton(page).click();
  return page.getByRole("list", { name: t("issues.change-status"), exact: true });
}

/** A word of a button in the open list. */
function word(page: Page, text: string) {
  return page.getByRole("button", { name: text, exact: true });
}

test("m4_f41 the list of a Bug has the words of the Bug workflow and no others", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const bug = await createBug(admin, "A bug that has just been reported");
  await admin.dispose();
  await signIn(page, ada);
  await openMenu(page, bug.key);
  await expect(word(page, serverWord("workflow-change.start-working"))).toBeVisible();
  await expect(word(page, serverWord("workflow-change.wont-fix"))).toBeVisible();
  // The Bug workflow has no way from Open to Done, and the list says words, not status names.
  await expect(word(page, serverWord("workflow-change.mark-as-done"))).toHaveCount(0);
  await expect(word(page, t("status.in-progress"))).toHaveCount(0);
});

test("wf_03 a person whose role may not make a change does not see it in the list", async ({
  page,
  browser,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const leaders = [
    await roleId(admin, roleName.administrator),
    await roleId(admin, roleName.projectLead),
  ];
  await changeSimple(admin, (content) =>
    onlyForRoles(content, { from: "open", to: "done" }, leaders),
  );
  await admin.dispose();
  await signIn(page, ada);
  await openMenu(page, "WEB-2");
  await expect(word(page, serverWord("workflow-change.mark-as-done"))).toBeVisible();
  const second = await anotherBrowser(browser, server);
  await signIn(second, bob);
  await openMenu(second, "WEB-2");
  await expect(word(second, serverWord("workflow-change.start-working"))).toBeVisible();
  await expect(word(second, serverWord("workflow-change.mark-as-done"))).toHaveCount(0);
  await expectStatus(second, t("status.open"));
});

test("wf_04 a change that needs the description says to write it first and has nothing to save", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  await changeSimple(admin, (content) =>
    mustFill(content, { from: "open", to: "done" }, ["description"]),
  );
  await admin.dispose();
  await signIn(page, ada);
  await openMenu(page, "WEB-3");
  await word(page, serverWord("workflow-change.mark-as-done")).click();
  const window = page.getByRole("dialog", { name: serverWord("workflow-change.mark-as-done") });
  await expect(window.getByText(t("issues.ask-for-description"))).toBeVisible();
  await expect(window.getByRole("textbox")).toHaveCount(0);
  await expect(
    window.getByRole("button", { name: serverWord("workflow-change.mark-as-done"), exact: true }),
  ).toHaveCount(0);
  await expectStatus(page, t("status.open"));
});
