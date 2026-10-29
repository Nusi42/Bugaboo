import { t } from "@bugaboo/messages";

import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { partIn } from "./support/parts.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { paths } from "../src/shared/paths.ts";

// M5 exit, package O: on a phone one group is a screen of its own, like a person: its name in the bar, Back to the
// list, and no card inside a card with a close button of its own.

test.use({ sample: true });

test("ux_5 on a phone a group opens as its own screen with its name in the bar", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "a computer opens a group beside the list");
  await signIn(page, sampleAdministrator());
  await page.goto(paths.groups);
  await page.getByRole("button", { name: "Backend", exact: true }).click();
  await expect(partIn(page, "top-bar.title")).toHaveText("Backend");
  await expect(partIn(page, "side-panel")).toHaveCount(0);
  // The title is in the bar; the heading of the card is for screen readers only.
  await expect(page.getByRole("heading", { level: 1 })).toHaveAttribute(
    "data-part",
    "visually-hidden",
  );
  await partIn(page, "top-bar")
    .getByRole("button", { name: t("shell.back") })
    .click();
  await expect(page).not.toHaveURL(/open=/u);
  await expect(page.getByRole("heading", { level: 1, name: t("nav.groups") })).toBeVisible();
});
