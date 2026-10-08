import { t } from "@bugaboo/messages";
import { type Locator, type Page } from "@playwright/test";

import { ada, bob, forgetChoices, makeAdmin, makePerson } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { addressEndingAt, anotherBrowser, sendPassword, signIn, signOut } from "./support/flows.ts";
import { paths } from "../src/shared/paths.ts";

// *My settings* (CFG-04, THM-02) and the settings of the organization (CFG-06) with a real server:
// what a person chooses is kept on the server, follows them to another browser, is gone from the
// browser after they sign out, and an administrator can lock it. Every test runs on a computer and
// on a phone.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await makeAdmin(server);
      await (await makePerson(server, admin, bob)).dispose();
      await admin.dispose();
    },
  },
});

test.beforeEach(async ({ server }) => {
  await forgetChoices(server, ada, true);
  await forgetChoices(server, bob);
});

/** Turns a switch on or off by pressing its words, as a person does. */
async function setSwitch(page: Page, name: string, on: boolean): Promise<void> {
  const field = page.getByRole("switch", { name, exact: true });
  if ((await field.isChecked()) !== on) {
    await page.getByText(name, { exact: true }).click();
  }
  await (on ? expect(field).toBeChecked() : expect(field).not.toBeChecked());
}

/** Chooses a radio button by pressing it, and waits until it shows as the chosen one. */
async function choose(radio: Locator): Promise<void> {
  await radio.click();
  await expect(radio).toBeChecked();
}

/** The words *Saved* are on the page: the server has the change. */
async function saved(page: Page): Promise<void> {
  await expect(page.getByRole("status").filter({ hasText: t("save.saved") })).toBeVisible();
}

/** Chooses *Always dark* and the theme Midnight, as a person does on *Appearance*. */
async function chooseMidnight(page: Page): Promise<void> {
  await page.goto(paths.appearance);
  await choose(page.getByRole("radio", { name: t("appearance.mode.dark"), exact: true }));
  const group = page.getByRole("group", { name: t("appearance.dark"), exact: true });
  await choose(group.getByRole("radio", { name: t("theme.midnight"), exact: true }));
  await expect(page.locator("html")).toHaveAttribute("data-look", "midnight");
  await saved(page);
}

/** Chooses a time zone on *Region*. */
async function chooseTimeZone(page: Page, zone: string): Promise<void> {
  await page.goto(paths.region);
  await page
    .getByRole("combobox", { name: t("setting.region.time-zone.name"), exact: true })
    .selectOption(zone);
  await saved(page);
}

/** Gives *Go home* the keys `g i` on *Keyboard shortcuts*. */
async function chooseKeys(page: Page): Promise<void> {
  await page.goto(paths.shortcuts);
  await page.getByRole("button", { name: t("shortcut.go-home"), exact: true }).click();
  const panel = page.getByRole("complementary", { name: t("shortcut.go-home"), exact: true });
  await panel.getByRole("textbox", { name: t("shortcuts.new-keys"), exact: true }).click();
  await page.keyboard.press("g");
  await page.keyboard.press("i");
  await panel.getByRole("button", { name: t("shortcuts.use") }).click();
  await expect(panel.getByText(t("shortcuts.saved"))).toBeVisible();
}

test("cfg_04 what a person chooses in My settings follows them to another browser", async ({
  page,
  browser,
  server,
}) => {
  await page.emulateMedia({ colorScheme: "light" });
  await signIn(page, ada);
  await chooseMidnight(page);
  await setSwitch(page, t("appearance.less-motion"), true);
  await setSwitch(page, t("appearance.larger-text"), true);
  await saved(page);
  await chooseTimeZone(page, "Pacific/Auckland");
  await chooseKeys(page);
  await page.keyboard.press("Escape");

  // A second browser of the same person: nothing was set there, and it looks the same.
  const other = await anotherBrowser(browser, server);
  await other.emulateMedia({ colorScheme: "light" });
  await signIn(other, ada);
  await expect(other.locator("html")).toHaveAttribute("data-look", "midnight");
  await expect(other.locator("html")).toHaveAttribute("data-motion", "less");
  await expect(other.locator("html")).toHaveAttribute("data-text", "larger");
  await other.goto(paths.region);
  await expect(other.getByLabel(t("setting.region.time-zone.name"), { exact: true })).toHaveValue(
    "Pacific/Auckland",
  );
  // The keys of the shortcut are the ones the person chose.
  await other.goto(paths.appearance);
  await expect(other.getByRole("heading", { level: 1, name: t("appearance.title") })).toBeVisible();
  await other.keyboard.press("g");
  await other.keyboard.press("i");
  await expect(other).toHaveURL(addressEndingAt(paths.home));
  await other.close();
});

test("cfg_04 after signing out nothing of the person's choices is left in the browser", async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: "light" });
  await signIn(page, ada);
  await chooseMidnight(page);
  await signOut(page);
  const kept = await page.evaluate(() => Object.keys(localStorage).join(","));
  expect(kept).not.toContain("first-frame");
  // The sign-in page is the device's: light on a light device.
  await expect(page.locator("html")).toHaveAttribute("data-look", "daylight");
  // The next person gets their own look, not the last person's.
  await sendPassword(page, bob);
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await expect(page.locator("html")).toHaveAttribute("data-look", "daylight");
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-look", "daylight");
});

/** What this browser keeps in its storage, as `key=value` lines. */
function storedText(page: Page): Promise<string> {
  return page.evaluate(() => {
    const lines: string[] = [];
    for (const store of [localStorage, sessionStorage]) {
      for (const key of Object.keys(store)) {
        lines.push(`${key}=${store.getItem(key) ?? ""}`);
      }
    }
    return lines.join(", ");
  });
}

test("cfg_04 the storage of the browser holds nothing of the person while they are signed in or after", async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: "light" });
  // What the device kept before the choices lived on the server (the look and the keys of the
  // pages before sign-in) is removed at sign-in.
  await page.goto(paths.signIn);
  await page.evaluate(() => {
    localStorage.setItem("bugaboo.appearance", JSON.stringify({ mode: "dark" }));
    localStorage.setItem("bugaboo.shortcut-keys", JSON.stringify({ "go-home": ["g x"] }));
  });
  await signIn(page, ada);
  await chooseMidnight(page);
  await chooseTimeZone(page, "Pacific/Auckland");
  await chooseKeys(page);
  const whileSignedIn = await storedText(page);
  expect(whileSignedIn).not.toContain("bugaboo.appearance");
  expect(whileSignedIn).not.toContain("bugaboo.shortcut-keys");
  expect(whileSignedIn).not.toContain("Pacific/Auckland");
  await signOut(page);
  // Nothing of the person is left in either storage.
  const left = await storedText(page);
  for (const word of ["midnight", "Pacific/Auckland", "go-home", "g i", ada.email, ada.name]) {
    expect(left).not.toContain(word);
  }
  expect(left).not.toContain("first-frame");
});

test("cfg_06 an administrator locks the theme and a person sees Set by your organization", async ({
  page,
  browser,
  server,
}) => {
  await page.emulateMedia({ colorScheme: "light" });
  await signIn(page, ada);
  await page.goto(paths.themes);
  const row = page.getByRole("listitem").filter({
    has: page.getByRole("group", { name: t("theme.legend"), exact: true }),
  });
  await choose(row.getByRole("radio", { name: t("appearance.mode.dark"), exact: true }));
  await saved(page);
  await row.getByText(t("setting.ui.lock"), { exact: true }).click();
  await expect(row.getByRole("switch", { name: t("setting.ui.lock") })).toBeChecked();
  await saved(page);

  const person = await anotherBrowser(browser, server);
  await person.emulateMedia({ colorScheme: "light" });
  await signIn(person, bob);
  await person.goto(paths.appearance);
  // The organization's theme counts, the choice is read only and the page says why.
  await expect(person.locator("html")).toHaveAttribute("data-look", "dusk");
  await expect(person.getByText(t("setting.ui.locked"), { exact: true }).first()).toBeVisible();
  await expect(person.getByText(t("setting.ui.locked-why")).first()).toBeVisible();
  await expect(
    person.getByRole("radio", { name: t("appearance.mode.light"), exact: true }),
  ).toHaveCount(0);
  await expectGoodInEveryLook(person);

  // Unlocked, the person chooses again.
  await row.getByText(t("setting.ui.lock"), { exact: true }).click();
  await expect(row.getByRole("switch", { name: t("setting.ui.lock") })).not.toBeChecked();
  await saved(page);
  await person.reload();
  await choose(person.getByRole("radio", { name: t("appearance.mode.light"), exact: true }));
  await expect(person.locator("html")).toHaveAttribute("data-look", "daylight");
  await person.close();
});

test("cfg_04 the start page is the page a person chose, after signing in", async ({ page }) => {
  await signIn(page, bob);
  await page.goto(paths.startPage);
  await page
    .getByRole("combobox", { name: t("nav.start-page"), exact: true })
    .selectOption({ label: `${t("nav.region")} (${t("nav.my-settings")})` });
  await saved(page);
  await signOut(page);
  await sendPassword(page, bob);
  await expect(page).toHaveURL(addressEndingAt(paths.region));
  await expect(page.getByRole("heading", { level: 1, name: t("nav.region") })).toBeVisible();
});

test("cfg_04 times follow the region the person chose", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.region);
  await choose(
    page.getByRole("radio", { name: t("setting.region.date-format.year-month-day"), exact: true }),
  );
  await saved(page);
  await page.goto(paths.sessions);
  await expect(page.getByText(/\d{4}-\d{2}-\d{2}, /u).first()).toBeVisible();
  // Reset to default brings the usual writing back.
  await page.goto(paths.region);
  await page
    .getByRole("button", {
      name: t("setting.ui.reset-name", { name: t("setting.region.date-format.name") }),
    })
    .click();
  await saved(page);
  await page.goto(paths.sessions);
  await expect(page.getByText(/\d{4}-\d{2}-\d{2}, /u)).toHaveCount(0);
});

test("cfg_06 the sign-in rules are drawn from the registry and saved at once", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.signInRules);
  const idle = t("setting.security.session-idle-minutes.name");
  // The two settings of access keys are on the page too, each with the bounds it accepts.
  const days = t("setting.ui.between", { least: 1, most: 365 });
  await expect(
    page.getByLabel(t("setting.auth.access-key-default-days.name"), { exact: true }),
  ).toHaveValue("90");
  await expect(
    page.getByLabel(t("setting.auth.access-key-longest-days.name"), { exact: true }),
  ).toHaveValue("365");
  await expect(page.getByText(days)).toHaveCount(2);
  await page.getByLabel(idle, { exact: true }).fill("45");
  await page
    .getByRole("button", { name: t("common.save") })
    .first()
    .click();
  await saved(page);
  await page.reload();
  await expect(page.getByLabel(idle, { exact: true })).toHaveValue("45");
  // A number outside its bounds, or none, is not sent: the box says what is allowed, and the old
  // value stays.
  const between = t("setting.ui.between", { least: 5, most: 1440 });
  const tryTyping = async (wrong: string): Promise<void> => {
    await page.getByLabel(idle, { exact: true }).fill(wrong);
    await page
      .getByRole("button", { name: t("common.save") })
      .first()
      .click();
    await expect(page.getByLabel(idle, { exact: true })).toHaveAttribute("aria-invalid", "true");
    await expect(page.getByText(between)).toHaveCount(2);
  };
  await tryTyping("1");
  await tryTyping("many");
  await page.reload();
  await expect(page.getByLabel(idle, { exact: true })).toHaveValue("45");
  // The search finds a setting by what it says.
  await page.getByRole("searchbox", { name: t("setting.ui.search") }).fill("two-step");
  await expect(page.getByText(t("setting.security.two-step-required.help"))).toBeVisible();
  await expect(page.getByLabel(idle, { exact: true })).toHaveCount(0);
  await page.getByRole("searchbox", { name: t("setting.ui.search") }).fill("zzz");
  await expect(page.getByText(t("setting.ui.nothing"))).toBeVisible();
  await expectGoodInEveryLook(page);
});
