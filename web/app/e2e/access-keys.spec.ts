import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, makeAdmin } from "./support/accounts.ts";
import { useKey } from "./support/api.ts";
import { expectGoodInEveryTheme } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { openFromMenu, signIn, signOut } from "./support/flows.ts";

test.use({
  preparation: {
    run: async (server) => {
      await (await makeAdmin(server)).dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

/** A route that lets people in who may invite people: the one these keys are tried on. */
const INVITATIONS = "/api/v1/auth/invitations";

const KEY_SHAPE = /^bb_[0-9a-z]{8}_[0-9a-z]{52}$/u;

/** What the form is made of: the part of the page that holds it. */
type Form = ReturnType<Page["getByRole"]>;

/** Presses the words next to a switch, as a person does. */
async function pressWords(form: Form, words: string): Promise<void> {
  await form.getByText(words, { exact: true }).click();
}

/** Ticks a box by pressing its words, and checks that it is ticked. */
async function tick(form: Form, words: string): Promise<void> {
  await pressWords(form, words);
  await expect(form.getByRole("checkbox", { name: words, exact: true })).toBeChecked();
}

/** Opens *Access keys* under *My settings*, the way a person does. */
async function openAccessKeys(page: Page): Promise<void> {
  await openFromMenu(page, t("nav.access-keys"));
  await expect(page.getByRole("heading", { level: 1, name: t("access-keys.title") })).toBeVisible();
}

/** Opens the form, fills in the name and the password, and leaves what else is picked to the test. */
async function startKey(page: Page, name: string) {
  await page.getByRole("button", { name: t("access-keys.make"), exact: true }).click();
  const form = page.getByRole("region", { name: t("access-keys.make"), exact: true });
  await form.getByLabel(t("access-keys.name")).fill(name);
  return form;
}

/** Presses *Make the key* after the password was typed again, and returns the key that is shown. */
async function makeKey(page: Page, form: Form): Promise<string> {
  await form.getByLabel(t("field.confirm-password"), { exact: true }).fill(ada.password);
  await form.getByRole("button", { name: t("access-keys.submit") }).click();
  await expect(page.getByRole("region", { name: t("access-keys.shown-title") })).toBeVisible();
  const key = await keyShown(page);
  await page
    .getByRole("region", { name: t("access-keys.shown-title") })
    .getByRole("button", { name: t("access-keys.done") })
    .click();
  return key;
}

/** The key that is shown, as text: it is read from the screen, as a person copies it. */
async function keyShown(page: Page): Promise<string> {
  const key = await page
    .getByRole("region", { name: t("access-keys.shown-title") })
    .locator("code")
    .innerText();
  expect(key).toMatch(KEY_SHAPE);
  return key;
}

/** Makes a key that may do everything, and copies it with the button; returns the key. */
async function makeAndCopy(page: Page): Promise<string> {
  const form = await startKey(page, "Nightly report");
  await form.getByRole("radio", { name: t("access-keys.choice.everything") }).check();
  await expectGoodInEveryTheme(page);
  await form.getByLabel(t("field.confirm-password"), { exact: true }).fill(ada.password);
  await form.getByRole("button", { name: t("access-keys.submit") }).click();
  const shown = page.getByRole("region", { name: t("access-keys.shown-title") });
  await expect(shown).toBeVisible();
  const key = await keyShown(page);
  await shown.getByRole("button", { name: t("access-keys.copy"), exact: true }).click();
  await expect(
    shown.getByRole("button", { name: t("access-keys.copied"), exact: true }),
  ).toBeVisible();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(key);
  await expectGoodInEveryTheme(page);
  return key;
}

/** The key is shown once: after *I have copied it* and after a reload only its start is listed. */
async function expectNotShownAgain(page: Page, key: string): Promise<void> {
  const shown = page.getByRole("region", { name: t("access-keys.shown-title") });
  await shown.getByRole("button", { name: t("access-keys.done") }).click();
  await expect(shown).toHaveCount(0);
  await expect(page.getByText(key)).toHaveCount(0);
  await page.reload();
  const list = page.getByRole("list", { name: t("access-keys.list") });
  await expect(list.getByText("Nightly report")).toBeVisible();
  await expect(
    list.getByText(t("access-keys.row.prefix", { prefix: key.slice(0, 11) })),
  ).toBeVisible();
  await expect(list.getByText(t("access-keys.choice.everything"))).toBeVisible();
  await expect(page.getByText(key)).toHaveCount(0);
  await expectGoodInEveryTheme(page);
}

/** Deleting asks first, takes the key away at once and says so. */
async function deleteTheKey(page: Page): Promise<void> {
  await page
    .getByRole("button", { name: t("access-keys.delete-name", { name: "Nightly report" }) })
    .click();
  await expectGoodInEveryTheme(page);
  await page
    .getByRole("alertdialog")
    .getByRole("button", { name: t("access-keys.delete-confirm") })
    .click();
  await expect(page.getByText(t("access-keys.deleted"))).toBeVisible();
  await expect(page.getByText(t("access-keys.empty"))).toBeVisible();
}

test("auth_07 a person makes a key, copies it, uses it from a script, deletes it, and the script is refused", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await openAccessKeys(page);
  await expect(page.getByText(t("access-keys.empty"))).toBeVisible();
  await expectGoodInEveryTheme(page);
  const key = await makeAndCopy(page);
  // A script that has the key works as the person.
  const script = await useKey(server.url, key);
  expect(await script.get(INVITATIONS)).toEqual({ status: 200, code: undefined });
  await expectNotShownAgain(page, key);
  await deleteTheKey(page);
  expect(await script.get(INVITATIONS)).toEqual({ status: 401, code: "auth.not_signed_in" });
  await script.dispose();
  await signOut(page);
});

test("auth_07 a key that may only see is refused what needs more, and single permissions can be picked", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await openAccessKeys(page);

  // The first choice is *See only*: inviting people is more than looking.
  const seeOnly = await makeKey(page, await startKey(page, "Only looking"));
  const lookingScript = await useKey(server.url, seeOnly);
  expect(await lookingScript.get(INVITATIONS)).toEqual({ status: 403, code: "access.forbidden" });

  // Under *More options*, one permission: inviting people. Nothing else.
  const form = await startKey(page, "Only inviting");
  await form.getByText(t("access-keys.more")).click();
  await pressWords(form, t("access-keys.single"));
  await tick(form, t("permission.manage-invitations"));
  await expectGoodInEveryTheme(page);
  const inviting = await makeKey(page, form);
  const invitingScript = await useKey(server.url, inviting);
  expect(await invitingScript.get(INVITATIONS)).toEqual({ status: 200, code: undefined });

  // The list says what each key may do, and how often a key was used.
  const list = page.getByRole("list", { name: t("access-keys.list") });
  await expect(list.getByText(t("access-keys.row.permissions", { count: 1 }))).toBeVisible();
  await expect(list.getByText(t("access-keys.choice.see-only"))).toBeVisible();
  await expect(list.getByText(t("access-keys.row.projects-all")).first()).toBeVisible();
  await page.reload();
  await expect(list.getByText(t("access-keys.row.never-used"))).toHaveCount(0);

  // A key can do nothing about the way its owner signs in.
  const own = await lookingScript.get("/api/v1/auth/sessions");
  expect(own).toEqual({ status: 403, code: "auth.access_key_not_allowed" });
  await lookingScript.dispose();
  await invitingScript.dispose();
  await signOut(page);
});

test("auth_07 the form asks for a name, a permission when single ones are chosen, and the password", async ({
  page,
}) => {
  await signIn(page, ada);
  await openAccessKeys(page);
  const form = await startKey(page, "A key");
  // Single permissions are on and none is ticked.
  await form.getByText(t("access-keys.more")).click();
  await pressWords(form, t("access-keys.single"));
  await form.getByLabel(t("field.confirm-password"), { exact: true }).fill(ada.password);
  await form.getByRole("button", { name: t("access-keys.submit") }).click();
  await expect(page.getByText(t("access-keys.pick-one"))).toBeVisible();
  // A wrong password: the server says so, and no key is made.
  await tick(form, t("permission.see-status"));
  await form.getByLabel(t("field.confirm-password"), { exact: true }).fill("Not-The-Password-1");
  await form.getByRole("button", { name: t("access-keys.submit") }).click();
  await expect(page.getByText(t("error.auth.wrong-credentials"))).toBeVisible();
  await expect(page.getByRole("region", { name: t("access-keys.shown-title") })).toHaveCount(0);
  await expectGoodInEveryTheme(page);
  await form.getByRole("button", { name: t("access-keys.cancel") }).click();
  await expect(form).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: t("access-keys.make"), exact: true }),
  ).toBeVisible();
  await signOut(page);
});
