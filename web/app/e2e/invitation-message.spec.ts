import { t } from "@bugaboo/messages";
import { type Locator } from "@playwright/test";

import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { paths } from "../src/shared/paths.ts";

// M5 exit, package O (O8): the message of a new invitation breaks at sensible points only: the address of
// the person is not cut before its last letter, on a narrow screen it moves to the next line as a whole.

test.use({ sample: true });

/** On how many lines the words of an element that start with the address are drawn. */
function linesOfAddress(message: Locator, address: string): Promise<number> {
  return message.evaluate((node, wanted) => {
    const text = node.firstChild;
    if (text === null) {
      return -1;
    }
    const at = (node.textContent ?? "").indexOf(wanted);
    const range = document.createRange();
    range.setStart(text, at);
    range.setEnd(text, at + wanted.length);
    const tops = new Set<number>();
    for (const rect of range.getClientRects()) {
      tops.add(Math.round(rect.top));
    }
    return tops.size;
  }, address);
}

test("ux_9 the address in the message of a new invitation stays in one piece", async ({ page }) => {
  const email = "new.colleague@bugaboo.example";
  await signIn(page, sampleAdministrator());
  await page.goto(paths.people);
  await page.getByLabel(t("field.email"), { exact: true }).fill(email);
  await page.getByRole("button", { name: t("invitations.create"), exact: true }).click();
  const message = page.getByText(t("invitations.created", { email }), { exact: true });
  await expect(message).toBeVisible();
  expect(await linesOfAddress(message, email), "lines the address is drawn on").toBe(1);
});
