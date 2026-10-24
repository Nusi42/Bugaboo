import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { openFromMenu, signIn } from "./support/flows.ts";

// The day a key runs out (M4, the final check): the day in the box of the form is the day the list
// of keys says afterwards, for a person whose calendar day is not the day in UTC. Two zones are
// used because one of them is on another day than UTC whatever the hour is: 14 hours ahead of UTC
// and 11 hours behind it.

test.use({
  preparation: {
    run: async (server) => {
      await (await createAdmin(server)).dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

/** The day as the list of keys writes it, for a day in the box (`2027-01-23`). */
function written(day: string): string {
  return new Intl.DateTimeFormat("en-US", { dateStyle: "long", timeZone: "UTC" }).format(
    new Date(`${day}T12:00:00Z`),
  );
}

/** The day after a day in the box (`2027-01-23` gives `2027-01-24`). */
function dayAfter(day: string): string {
  const next = new Date(`${day}T12:00:00Z`);
  next.setUTCDate(next.getUTCDate() + 1);
  return next.toISOString().slice(0, "2027-01-24".length);
}

/** Makes a key as a person does and returns the day in the box, and whether the day was typed in. */
async function createKeyLikeAPerson(page: Page, name: string, typed: boolean): Promise<string> {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.access-keys"));
  await page.getByRole("button", { name: t("access-keys.create"), exact: true }).click();
  const form = page.getByRole("region", { name: t("access-keys.create"), exact: true });
  await form.getByLabel(t("access-keys.name")).fill(name);
  const box = form.getByLabel(t("access-keys.expires"), { exact: true });
  const preset = await box.inputValue();
  // A day typed in is another day than the one filled in: the same text would change nothing.
  const day = typed ? dayAfter(preset) : preset;
  if (typed) {
    await box.fill(day);
  }
  await form.getByLabel(t("field.confirm-password"), { exact: true }).fill(ada.password);
  await form.getByRole("button", { name: t("access-keys.submit") }).click();
  await page
    .getByRole("region", { name: t("access-keys.shown-title") })
    .getByRole("button", { name: t("access-keys.done") })
    .click();
  return day;
}

for (const timezoneId of ["Pacific/Kiritimati", "Pacific/Pago_Pago"]) {
  test.describe(`in ${timezoneId}`, () => {
    test.use({ timezoneId });

    for (const [how, typed] of [
      ["the day that is filled in", false],
      ["a day that was typed", true],
    ] as const) {
      test(`auth_07 the key runs out on ${how}`, async ({ page }) => {
        const name = `Key for ${timezoneId}, ${how}`;
        const day = await createKeyLikeAPerson(page, name, typed);
        await expect(
          page
            .getByRole("listitem")
            .filter({ hasText: name })
            .getByText(t("access-keys.row.expires", { when: written(day) }), { exact: true }),
        ).toBeVisible();
      });
    }
  });
}
