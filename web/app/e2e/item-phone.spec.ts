import { t } from "@bugaboo/messages";
import { expect } from "@playwright/test";

import { test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { partIn } from "./support/parts.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { inOrder } from "./support/sequence.ts";
import { paths } from "../src/shared/paths.ts";

// M5 exit, package O: on a phone an item of any list (a group, an issue type, a kind of relation) opens as a screen of
// its own, like a person: its name in the bar, Back to the list, and no card inside a card with a close button of its own.

test.use({ sample: true });

const lists = [paths.groups, paths.issueTypes, paths.relationKinds];

test("ux_5 on a phone an item of a list opens as its own screen with its name in the bar", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "a computer opens an item beside the list");
  await signIn(page, sampleAdministrator());
  await inOrder(
    lists.map((address) => async () => {
      await page.goto(address);
      const row = partIn(page, "list-editor.open").first();
      const name = await partIn(page, "list-editor.name").first().innerText();
      await row.click();
      await expect(partIn(page, "top-bar.title"), address).toHaveText(name);
      await expect(partIn(page, "side-panel"), address).toHaveCount(0);
      // The title is in the bar; the heading of the card is for screen readers only.
      await expect(page.getByRole("heading", { level: 1 })).toHaveAttribute(
        "data-part",
        "visually-hidden",
      );
      await partIn(page, "top-bar")
        .getByRole("button", { name: t("shell.back") })
        .click();
      await expect(page).not.toHaveURL(/open=/u);
    }),
  );
});
