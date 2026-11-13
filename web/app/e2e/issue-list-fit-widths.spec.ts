import type { Page } from "@playwright/test";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { signInThroughApi } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { openIssuesIn } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { chooseSpacing } from "./support/list-look.ts";
import { sampleAdministrator } from "./support/sample.ts";

// A list that hides columns to fit must fit (UX §4.2): the list of a project of the sample, with
// more rows than the window holds, grouped by status group, beside the open issue or not, at the
// usual window widths and in each *Spacing*. Neither the frame of the list nor the box its rows
// scroll in is wider inside than outside. The browser draws its scroll bars as a computer with a
// mouse does (Playwright hides them by default): the bar of the rows takes room from the columns.

test.use({ sample: true, launchOptions: { ignoreDefaultArgs: ["--hide-scrollbars"] } });

/** How far each box of the list scrolls sideways: its frame, and the box its rows scroll in. */
function sideways(page: Page): Promise<number[]> {
  return page
    .locator("[data-part='list-frame'], [data-part='list-scroll']")
    .evaluateAll((boxes: readonly HTMLElement[]) =>
      boxes.map((box) => box.scrollWidth - box.clientWidth),
    );
}

const cases = (["comfortable", "compact"] as const).flatMap((spacing) =>
  [true, false].flatMap((open) =>
    [1280, 1440, 1600].map((width) => ({
      spacing,
      width,
      where: open ? "beside the open issue" : "alone",
      opened: open ? "&open=BLOG-59" : "",
    })),
  ),
);

for (const { spacing, width, where, opened } of cases) {
  test(`ux_4_2 a list that hides columns does not scroll sideways: ${spacing}, ${where}, ${String(width)} px`, async ({
    page,
    server,
  }) => {
    test.skip(layoutOf(page) !== "computer", "only a computer has the panel beside the list");
    const person = sampleAdministrator();
    await chooseSpacing(server, spacing, person);
    const session = await signInThroughApi(server, person);
    await openIssuesIn(session, "panel");
    await session.dispose();
    await signIn(page, person);
    await page.setViewportSize({ width, height: 800 });
    await page.goto(`${projectIssuesAddress("BLOG")}?group=statusGroup${opened}`);
    await expect(
      page.locator("[data-part='list-row'][data-variant~='grid']").first(),
    ).toBeVisible();
    const found = await sideways(page);
    expect(found).not.toHaveLength(0);
    expect(Math.max(...found)).toBeLessThanOrEqual(0);
    // The ID always stays.
    await expect(
      page.locator("[data-part='list-row.cell'][data-variant~='key']").first(),
    ).toBeVisible();
  });
}
