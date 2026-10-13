import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, bob, makeAdmin, makePerson } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { openFromMenu, signIn } from "./support/flows.ts";
import { onThisLayout } from "./support/layout.ts";
import { makeProject } from "./support/projects.ts";
import { inOrder } from "./support/sequence.ts";

// Own fields (CF-01 to CF-08, WP-05): an administrator makes *Severity* (pick one, four colored
// choices), *Platforms* (pick several) and *Customer* (text) on the screens, they are filled in on
// an issue, *Severity* is hidden from a Guest, and the screens of fields are good in the four
// themes.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await makeAdmin(server);
      await (await makePerson(server, admin, bob)).dispose();
      await makeProject(admin, "WEB", "Website");
      await admin.dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

/** Makes a field on the screen of fields: the kind, the name and, for choices, one on each line. */
async function makeField(
  page: Page,
  kind: string,
  name: string,
  choices: readonly string[] = [],
): Promise<void> {
  await openFromMenu(page, t("nav.fields"));
  await page.getByRole("radio", { name: kind, exact: true }).check();
  await page.getByLabel(t("fields.field-name"), { exact: true }).fill(name);
  if (choices.length > 0) {
    await page.getByLabel(t("fields.field-choices"), { exact: true }).fill(choices.join("\n"));
  }
  await page.getByRole("button", { name: t("fields.make"), exact: true }).click();
  await expect(page.getByRole("heading", { level: 1, name })).toBeVisible();
}

/** The four choices of *Severity* with the color each is given. */
const severity = [
  ["Minor", "cat3"],
  ["Major", "cat4"],
  ["Critical", "cat1"],
  ["Blocker", "cat7"],
] as const;

test("cf_04 an administrator makes Severity with four colored choices and sees it in all four themes", async ({
  page,
}) => {
  await signIn(page, ada);
  await makeField(
    page,
    t("field-type.pick-one"),
    "Severity",
    severity.map(([name]) => name),
  );
  await inOrder(
    severity.map(([name, color]) => async () => {
      const group = page.getByRole("group", { name: t("fields.choice-color", { name }) });
      const swatch = group.getByRole("radio", { name: t(`choice-color.${color}`), exact: true });
      await swatch.click();
      await expect(swatch).toBeChecked();
    }),
  );
  // The look shows the four themes side by side, and every look is readable in each of them.
  await expect(page.getByRole("group", { name: t("fields.look-preview") })).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("cf_01 Platforms (pick several) and Customer (text) are made on the same screen", async ({
  page,
}) => {
  await signIn(page, ada);
  await makeField(page, t("field-type.pick-several"), "Platforms", ["Windows", "Mac", "Linux"]);
  await makeField(page, t("field-type.text"), "Customer");
  await openFromMenu(page, t("nav.fields"));
  const list = page.getByRole("list", { name: t("fields.list") });
  await expect(list.getByRole("link")).toHaveText(["Severity", "Platforms", "Customer"]);
  await expectGoodInEveryLook(page);
});

/** The choices of a field in the order they are listed in: the first words of each. */
function rowsOfChoices(page: Page) {
  return page.getByRole("list", { name: t("fields.choices") }).locator(":scope > li");
}

test("m3_f61 the choices of a field are put in order with the buttons, and by dragging on a computer", async ({
  page,
}) => {
  await signIn(page, ada);
  await makeField(page, t("field-type.pick-one"), "Impact", ["Low", "Mid", "High"]);
  const rows = rowsOfChoices(page);
  await expect(rows.first()).toContainText("Low");
  // The buttons do everything a drag does: High goes up one place.
  await page.getByRole("button", { name: t("fields.choice-up", { name: "High" }) }).click();
  await expect(rows.nth(1)).toContainText("High");
  // Mid is dragged in front of the first choice, where a window is wide enough for a handle.
  const dragMid = async (): Promise<void> => {
    await page
      .getByRole("button", { name: t("fields.choice-drag", { name: "Mid" }) })
      .dragTo(rows.first().locator('[data-part="drop-slot"]').first());
    await expect(rows.first()).toContainText("Mid");
  };
  await onThisLayout(page, {
    computer: dragMid,
    tablet: dragMid,
    phone: async () => {
      await expect(
        page.getByRole("button", { name: t("fields.choice-drag", { name: "Mid" }) }),
      ).toHaveCount(0);
    },
  });
});
