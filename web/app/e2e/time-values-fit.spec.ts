import { settingKeys } from "@bugaboo/api-client";
import { type Page } from "@playwright/test";

import { ana } from "./support/access.ts";
import { signInThroughApi, type Person } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { hana } from "./support/hr.ts";
import { sheetAt } from "./support/timesheet.ts";
import { anaWorked, PAST, prepareTimesheet } from "./support/timesheet-data.ts";

// A time in a box of the week is written whole in every Region: the 12-hour clock with AM or PM is the longest.

test.use({ preparation: { run: prepareTimesheet } });

/** Every box of the open week whose text does not fit in it. */
function clippedBoxes(page: Page): Promise<string[]> {
  return page
    .getByRole("textbox")
    .evaluateAll((boxes: readonly (HTMLElement | SVGElement)[]) =>
      boxes.flatMap((box) =>
        box instanceof HTMLInputElement && box.value !== "" && box.scrollWidth > box.clientWidth
          ? [box.value]
          : [],
      ),
    );
}

/** How far a grid of the week has to scroll sideways (none or less; a phone shows no grid). */
function sidewaysOverflow(page: Page): Promise<number> {
  return page
    .locator('[data-part="sheet.scroll"]')
    .evaluateAll((grids: readonly (HTMLElement | SVGElement)[]) =>
      Math.max(0, ...grids.map((grid) => grid.scrollWidth - grid.clientWidth)),
    );
}

/** The person writes times on the 12-hour clock. */
async function chooseTwelveHours(server: Parameters<typeof signInThroughApi>[0], person: Person) {
  const session = await signInThroughApi(server, person);
  await session.client.PUT("/api/v1/me/settings/{key}", {
    params: { path: { key: settingKeys.regionTimeOfDay } },
    body: { value: "12-hour" },
  });
  await session.dispose();
}

test("time_12 a time written on the 12-hour clock fits its box, in the week of another person", async ({
  page,
  server,
}) => {
  const anaId = await anaWorked(server);
  await chooseTwelveHours(server, hana);
  await signIn(page, hana);
  await page.goto(`/time/people/${anaId}${sheetAt(PAST).slice("/time".length)}`);
  await expect(page.getByRole("textbox").first()).toHaveValue(/(AM|PM)$/u);
  expect(await clippedBoxes(page)).toEqual([]);
  expect(await sidewaysOverflow(page)).toBe(0);
});

test("time_04 a time written on the 12-hour clock fits its box, in the week of the person", async ({
  page,
  server,
}) => {
  await anaWorked(server);
  await chooseTwelveHours(server, ana);
  await signIn(page, ana);
  await page.goto(sheetAt(PAST));
  await expect(page.getByRole("textbox").first()).toHaveValue(/(AM|PM)$/u);
  expect(await clippedBoxes(page)).toEqual([]);
  expect(await sidewaysOverflow(page)).toBe(0);
});
