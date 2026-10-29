import { t } from "@bugaboo/messages";
import { type Locator } from "@playwright/test";

import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { paths } from "../src/shared/paths.ts";

// M5 exit, package O (O8): the message of a new invitation breaks at sensible points only: the address of
// the person is not cut before its last letter, on a narrow screen it moves to the next line as a whole.

test.use({ sample: true });

/** How many lines of the address end in the middle of a word: not after its `@` or a dot. */
function breaksInsideWords(message: Locator, address: string): Promise<number> {
  return message.evaluate((node, wanted) => {
    const found = document.createTreeWalker(node, NodeFilter.SHOW_TEXT);
    const spots: { node: Text; at: number; char: string }[] = [];
    let all = "";
    for (let text = found.nextNode(); text instanceof Text; text = found.nextNode()) {
      const content = text.data;
      for (let at = 0; at < content.length; at += 1) {
        spots.push({ node: text, at, char: content.charAt(at) });
        all += content.charAt(at);
      }
    }
    const start = all.indexOf(wanted);
    let bad = 0;
    for (let at = start; at < start + wanted.length - 1; at += 1) {
      const here = spots[at];
      const next = spots[at + 1];
      if (here !== undefined && next !== undefined) {
        const one = document.createRange();
        one.setStart(here.node, here.at);
        one.setEnd(here.node, here.at + 1);
        const two = document.createRange();
        two.setStart(next.node, next.at);
        two.setEnd(next.node, next.at + 1);
        const lower =
          Math.round(two.getBoundingClientRect().top) > Math.round(one.getBoundingClientRect().top);
        if (lower && here.char !== "@" && here.char !== ".") {
          bad += 1;
        }
      }
    }
    return bad;
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
  expect(await breaksInsideWords(message, email), "breaks inside a word").toBe(0);
});
