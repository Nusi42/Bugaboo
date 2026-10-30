import { t } from "@bugaboo/messages";

import { issueAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { dropFile, filesBlock, report, rowOf } from "./support/files.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareLook } from "./support/list-look.ts";
import { partOn, resolved } from "./support/styles.ts";
import { linesOf } from "./support/windows.ts";
import { expectNameOnOneLine, expectSentenceOnOneLine } from "./support/name-lines.ts";

// A file name never breaks at a hyphen (the final check of M4): in the row of the file and in the
// sentence of the history it goes to the next line as a whole, on a phone too.

test.use({ preparation: { run: prepareLook } });

test("ux_4_4 the name quarter-report.pdf stays on one line in its row and in the history", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-2"));
  await dropFile(
    filesBlock(page).getByRole("button", { name: t("files.drop-place"), exact: true }),
    report(),
  );
  await expect(rowOf(page, "quarter-report.pdf")).toBeVisible();
  await expectNameOnOneLine(page, "quarter-report.pdf", 2);
  await expectSentenceOnOneLine(
    page,
    t("history.file-added", { who: ada.name, name: "quarter-report.pdf" }),
  );
});

test("ux_4_4 the count of lines sees a name that breaks at its hyphen", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-2"));
  // A name in a box too narrow for it, without the part that keeps it whole: it must be two lines.
  await page.evaluate(() => {
    const box = document.createElement("div");
    box.dataset["probe"] = "narrow-name";
    box.style.width = "6ch";
    box.textContent = "quarter-report.pdf";
    document.body.append(box);
  });
  expect(await linesOf(page.locator('[data-probe="narrow-name"]'))).toBeGreaterThan(1);
});

test("ux_4_4 the place to drop files has the edge of a control, 3:1 like a text box (M5 exit F-A11)", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  const drop = partOn(page, "files.drop").first();
  await expect(drop).toBeVisible();
  await expect(drop).toHaveCSS(
    "border-top-color",
    await resolved(page, "border-top-color", "var(--border-control)"),
  );
});
