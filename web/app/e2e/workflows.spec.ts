import { t } from "@bugaboo/messages";

import { issueAddress } from "../src/shared/paths.ts";
import { ada, cleo } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { startWorking, statusButton } from "./support/issues.ts";
import { onThisLayout } from "./support/layout.ts";
import { askedForField, prepareWorkflows } from "./support/workflows.ts";
import {
  addSendToTesting,
  blockOf,
  openSimple,
  openSimpleToChange,
  requireAskedForField,
  startWorkingArrow,
  tryMarkAsDone,
} from "./support/workflows-pages.ts";

// The workflow editor and the status changes on screen (WF-02, WF-04, M4 WP-07, UX §5.4): a lead
// adds a status and a status change with a rule to *Simple*, publishes it, and tries it; the
// issue then asks for what the change needs.

test.use({ preparation: { run: prepareWorkflows } });
test.describe.configure({ mode: "serial" });

test("wf_02 a lead adds Testing to Simple with Must be filled in, publishes it, and the issue asks for it", async ({
  page,
}) => {
  await signIn(page, ada);
  await openSimpleToChange(page);
  await addSendToTesting(page);
  await requireAskedForField(page);
  await expectGoodInEveryLook(page);
  await page.getByRole("button", { name: t("workflows.publish"), exact: true }).click();
  await expect(page.getByText(t("save.saved"), { exact: true })).toBeVisible();
  await page.goto(issueAddress("WEB-2"));
  await startWorking(page);
  await statusButton(page).click();
  await page.getByRole("button", { name: "Send to testing", exact: true }).click();
  const asking = page.getByRole("dialog", { name: "Send to testing" });
  await expect(asking.getByRole("textbox")).toHaveCount(1);
  await asking.getByRole("button", { name: "Send to testing", exact: true }).click();
  await expect(asking.getByText(t("error.field.required"), { exact: true })).toBeVisible();
  await asking.getByLabel(askedForField, { exact: true }).fill("Acme");
  await asking.getByRole("button", { name: "Send to testing", exact: true }).click();
  await expect(statusButton(page)).toHaveText("Testing");
  await expect(
    page.getByRole("button", { name: t("common.undo"), exact: true }).last(),
  ).toBeVisible();
});

test("wf_02 Try it names why a Contributor gets no Mark as done", async ({ page }) => {
  await signIn(page, ada);
  await openSimple(page);
  const markAsDone = await tryMarkAsDone(page, "WEB-1", cleo);
  await expect(markAsDone).toContainText(t("workflows.try-not-for-you"));
});

test("wf_02 the whole organization has nothing to inherit, so there is no way back to it", async ({
  page,
}) => {
  await signIn(page, ada);
  await openSimple(page);
  await expect(
    page.getByRole("button", { name: t("workflows.use-inherited"), exact: true }),
  ).toHaveCount(0);
});

test("wf_02 a status with a name another has is refused at the field, by the rule of the server", async ({
  page,
}) => {
  await signIn(page, ada);
  await openSimpleToChange(page);
  const name = page.getByLabel(t("workflows.new-status"), { exact: true });
  const create = page.getByRole("button", { name: t("workflows.create-status"), exact: true });
  // Capitals and the spaces inside a name make no difference.
  await name.fill("in   PROGRESS");
  await expect(page.getByText(t("workflows.status-name-taken"))).toBeVisible();
  await expect(create).toBeDisabled();
  await name.fill("Waiting");
  await expect(page.getByText(t("workflows.status-name-taken"))).toHaveCount(0);
  await expect(create).toBeEnabled();
});

test("wf_02 the phone shows the plain list and a computer may also draw", async ({ page }) => {
  await signIn(page, ada);
  await openSimpleToChange(page);
  await expect(
    page.getByRole("heading", { level: 2, name: t("workflows.statuses") }),
  ).toBeVisible();
  const choice = page.getByRole("group", { name: t("workflows.view") });
  await onThisLayout(page, {
    computer: async () => {
      await choice.getByRole("radio", { name: t("workflows.view-drawing"), exact: true }).check();
      await startWorkingArrow(page).click();
      await expect(page.getByRole("dialog")).toBeVisible();
    },
    tablet: async () => {
      await expect(choice).toHaveCount(1);
    },
    phone: async () => {
      await expect(choice).toHaveCount(0);
      await expect(blockOf(page, "Open")).toBeVisible();
    },
  });
});
