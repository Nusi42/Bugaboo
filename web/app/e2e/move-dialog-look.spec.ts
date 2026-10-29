import { t } from "@bugaboo/messages";
import { type Locator } from "@playwright/test";

import { ada, createAdmin } from "./support/accounts.ts";
import { answerPreviewsWithEverything } from "./support/fake-preview.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { partIn } from "./support/parts.ts";
import { createProject, openMove } from "./support/projects.ts";
import { boxOf } from "./support/windows.ts";
import { addressInProject, paths } from "../src/shared/paths.ts";

// M5 exit, package O (O1): the question *Move* with a long preview keeps its title and its answers on the
// screen, and the body between them scrolls; the last line of what changes can be reached and is not
// hidden behind the buttons.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      const website = await createProject(admin, "WEB", "Website");
      await createProject(admin, "DOCS", "Documents", website);
      await admin.dispose();
    },
  },
});

/** How much of the shadow at the top and at the end of a box shows: 1 where more is behind it, 0 where it ends. */
function shadowsOf(box: Locator): Promise<{ readonly top: number; readonly end: number }> {
  return box.evaluate((node) => ({
    top: Math.round(Number(getComputedStyle(node, "::before").opacity)),
    end: Math.round(Number(getComputedStyle(node, "::after").opacity)),
  }));
}

/** Scrolls an element to its end and says whether there was anything to scroll. */
function scrollToEnd(element: Locator): Promise<boolean> {
  return element.evaluate((node) => {
    const more = node.scrollHeight > node.clientHeight;
    node.scrollTop = node.scrollHeight;
    return more;
  });
}

test("ux_6 the question Move stays on the screen, its title and answers in view and its body scrolls", async ({
  page,
}) => {
  await signIn(page, ada);
  await answerPreviewsWithEverything(page);
  const dialog = await openMove(page, addressInProject(paths.projectSettings, "DOCS"));
  const screenHeight = await page.evaluate(() => window.innerHeight);
  const box = await boxOf(dialog);
  expect(box.y).toBeGreaterThanOrEqual(0);
  expect(box.y + box.height).toBeLessThanOrEqual(screenHeight + 0.5);
  const body = partIn(dialog, "confirm.body");
  const title = dialog.getByRole("heading", {
    name: t("projects.move-dialog-title", { name: "Documents" }),
  });
  const before = await boxOf(title);
  // At the start there is more below (the shadow at the end shows) and nothing above.
  await expect.poll(() => shadowsOf(body)).toEqual({ top: 0, end: 1 });
  expect(await scrollToEnd(body), "the body has more than it shows").toBe(true);
  // The title is not in the body that scrolls: it does not move by a pixel, and the body casts a shadow at each end.
  expect(await boxOf(title)).toEqual(before);
  await expect(body.getByRole("heading", { level: 2 })).toHaveCount(0);
  // Scrolled to the end: more is above (the shadow at the top shows) and the one at the end is gone.
  await expect.poll(() => shadowsOf(body)).toEqual({ top: 1, end: 0 });
  // The line under the title and the line above the answers are one color, the divider (its contrast is a rule of the theme).
  const line = await title.evaluate((node) => getComputedStyle(node).borderBottomColor);
  await expect(partIn(dialog, "confirm.actions")).toHaveCSS("border-top-color", line);
  // The title stays at the top of the question, whatever the body has scrolled: within its first line or two.
  const titleBox = await boxOf(title);
  expect(titleBox.y).toBeGreaterThanOrEqual(box.y);
  expect(titleBox.y).toBeLessThan(box.y + 80);
  // A line under the title and a line above the answers end what scrolls between them.
  await expect(title).toHaveCSS("border-bottom-width", "1px");
  await expect(partIn(dialog, "confirm.actions")).toHaveCSS("border-top-width", "1px");
  const answers = await boxOf(partIn(dialog, "confirm.actions"));
  const bodyBox = await boxOf(body);
  expect(bodyBox.y + bodyBox.height).toBeLessThanOrEqual(answers.y + 0.5);
  const last = await boxOf(dialog.locator("li").last());
  expect(last.y + last.height).toBeLessThanOrEqual(answers.y + 0.5);
});
