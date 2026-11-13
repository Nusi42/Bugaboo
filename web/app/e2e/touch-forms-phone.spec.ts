import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { newIssueDialog, openNewIssueDialog } from "./support/issues.ts";
import { prepareLook } from "./support/list-look.ts";
import { partIn } from "./support/parts.ts";
import { pixelsOf } from "./support/tokens.ts";
import { boxOf } from "./support/windows.ts";
import { projectIssuesAddress } from "../src/shared/paths.ts";

// The footer of a form on a phone keeps its gaps and its one start edge (UX-02, M6-F121, R1, R2): the buttons that a
// phone keeps out of the fixed footer stand the gap of fields under the last field, apart from each other, and the
// word of the quiet *Cancel* starts on the edge of the note under it, not in the middle.

test.use({ preparation: { run: prepareLook } });

// Only the new-issue form keeps buttons out of its fixed footer on a phone; the footers of the other windows are
// M8-F110.
test.describe("in the new-issue form", () => {
  test("ux_02_phone_form_footers_keep_their_gaps", async ({ page }) => {
    await signIn(page, ada);
    await page.goto(projectIssuesAddress("WEB"));
    await openNewIssueDialog(page);
    const dialog = newIssueDialog(page);
    const buttons = partIn(dialog, "window.more-actions");
    const another = buttons.getByRole("button", { name: t("issues.create-another"), exact: true });
    const cancel = buttons.getByRole("button", { name: t("common.cancel"), exact: true });
    await another.scrollIntoViewIfNeeded();
    const [above, first, second, note] = [
      await boxOf(buttons.locator("xpath=preceding-sibling::*[1]")),
      await boxOf(another),
      await boxOf(cancel),
      await boxOf(buttons.getByText(t("issues.draft-kept"), { exact: true })),
    ];
    expect(first.y - (above.y + above.height)).toBeGreaterThanOrEqual(
      await pixelsOf(page, "--gap-fields"),
    );
    expect(second.y - (first.y + first.height)).toBeGreaterThanOrEqual(
      await pixelsOf(page, "--space-2"),
    );
    // *Cancel* ends on the edge of the button above it, and its word starts on the edge of the note under it.
    expect(Math.abs(second.x + second.width - (first.x + first.width))).toBeLessThanOrEqual(1);
    const word = await cancel.evaluate((node) => {
      const words = document.createRange();
      words.selectNodeContents(node);
      return words.getBoundingClientRect().x;
    });
    expect(Math.abs(word - note.x)).toBeLessThanOrEqual(1);
  });
});
