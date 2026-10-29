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
  expect(await scrollToEnd(body), "the body has more than it shows").toBe(true);
  // The title stays in view, and the body and its last words end above the answers.
  const title = dialog.getByRole("heading", {
    name: t("projects.move-dialog-title", { name: "Documents" }),
  });
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
