import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { lea, signedInAdmin } from "./support/access.ts";
import { ada, bob, idOf, signInThroughApi } from "./support/accounts.ts";
import { createAgent } from "./support/agents-scene.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { expectStill, landmarkOf, movementWhile } from "./support/movement.ts";
import { openPersonPage, preparePrivacy, removalWindow, stateCard } from "./support/privacy.ts";

// *Switch off*, *Switch on* and *Remove a person* on a person's page (PRIV-04; UX §5.7; ADR-0044, decisions 5 and 6).

test.use({ preparation: { run: preparePrivacy } });

test("priv_04 switch off names what stops and the person's agent stops", async ({
  page,
  server,
}) => {
  const session = await signInThroughApi(server, bob);
  const agent = await createAgent(session, "Bob's helper");
  await signIn(page, ada);
  await openPersonPage(page, server, bob);
  const card = stateCard(page);
  await expect(card).toContainText(t("person.active", { name: bob.name }));
  await card.getByRole("button", { name: t("person.switch-off"), exact: true }).click();
  const question = page.getByRole("alertdialog");
  await expect(question).toContainText(t("person.switch-off-text", { name: bob.name }));
  await question.getByRole("button", { name: t("person.switch-off"), exact: true }).click();
  await expect(card).toContainText(t("person.switched-off", { name: bob.name }));
  // Bob's sign-in ended at once, and his agent stopped.
  await expect(session.client.GET("/api/v1/auth/me")).rejects.toThrow();
  const admin = await signedInAdmin(server);
  const agents = dataOf(await admin.client.GET("/api/v1/organization/agents"));
  expect(agents.items.find((each) => each.id === agent)?.state).toBe("stopped");
  // *Switch on* gives the sign-in back.
  await card.getByRole("button", { name: t("person.switch-on"), exact: true }).click();
  await expect(card).toContainText(t("person.active", { name: bob.name }));
  // Bob's session ended with the switch-off: there is nothing to sign out of.
  await admin.dispose();
});

test("priv_04 the removal window says what goes what stays and until when", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await openPersonPage(page, server, lea);
  await stateCard(page)
    .getByRole("button", { name: t("person.remove") })
    .click();
  const window = removalWindow(page, lea.name);
  const remove = window.getByRole("button", { name: t("removal.confirm", { name: lea.name }) });
  // Only a switched-off person is removed: the window offers to switch her off first.
  await expect(window).toContainText(t("removal.switch-off-first", { name: lea.name }));
  await expect(remove).toBeDisabled();
  await Promise.all(
    [t("removal.deleted-now"), t("removal.stays"), t("removal.kept-by-law")].map((heading) =>
      expect(window.getByRole("heading", { name: heading, exact: true })).toBeVisible(),
    ),
  );
  // Each kind is named in plain words before its count, never by a sentence of the code.
  const deleted = window
    .getByRole("list", { name: t("removal.deleted-now") })
    .getByRole("listitem");
  await expect(deleted.first()).toBeVisible();
  const lines = await Promise.all(
    [t("removal.deleted-now"), t("removal.stays")].map((heading) =>
      window.getByRole("list", { name: heading }).getByRole("listitem").allTextContents(),
    ),
  );
  for (const line of lines.flat()) {
    expect(line).toMatch(/^[^.:()]+: \d+ records?$/u);
  }
  // Her work day is kept by law, until a day the window names.
  await expect(window.getByRole("list", { name: t("removal.kept-by-law") })).toContainText(
    t("keep-kind.name.work-days"),
  );
  await expect(
    window.getByRole("radio", { name: t("setting.privacy.removed-texts.keep") }),
  ).toBeChecked();
  await window.getByRole("button", { name: t("removal.switch-off-button"), exact: true }).click();
  // Removing asks for the password again.
  await expect(remove).toBeDisabled();
  await window.getByLabel(t("field.confirm-password"), { exact: true }).fill(ada.password);
  await expect(remove).toBeEnabled();
  await window.getByRole("button", { name: t("common.cancel"), exact: true }).click();
  await expect(window).toBeHidden();
});

test("priv_04 after removal the person reads Former user in place", async ({ page, server }) => {
  const admin = await signedInAdmin(server);
  await admin.client.POST("/api/v1/people/{person}/switch-off", {
    params: { path: { person: await idOf(server, bob) } },
  });
  await signIn(page, ada);
  await openPersonPage(page, server, bob);
  const card = stateCard(page);
  // A person switched off before is removed at once from the window.
  await expect(card).toContainText(t("person.switched-off", { name: bob.name }));
  await card.getByRole("button", { name: t("person.remove") }).click();
  const window = removalWindow(page, bob.name);
  const heading = page.getByRole("heading", { level: 1 });
  // The page stands at its top behind the window, where the person reads it after the removal: the parts that go make
  // it shorter than a place further down.
  const headCard = page.locator('[data-part="card"]').first();
  await headCard.evaluate((element) => {
    element.scrollIntoView({ block: "end" });
  });
  await heading.evaluate((element) => {
    element.scrollIntoView({ block: "end" });
  });
  // The card under the heading holds the facts, and then what a removed person's page says.
  const landmarks = [await landmarkOf(heading, "heading"), await landmarkOf(headCard, "head")];
  await window.getByLabel(t("field.confirm-password"), { exact: true }).fill(ada.password);
  const movement = await movementWhile(page, landmarks, async () => {
    await window.getByRole("button", { name: t("removal.confirm", { name: bob.name }) }).click();
    await expect(heading).toHaveText(/^Former user [A-Z0-9]{4}$/u);
  });
  await expect(window).toBeHidden();
  await expect(page.getByText(t("person.former-title"))).toBeVisible();
  // The heading takes the new name where the old one stood, and the card under it its new words.
  expectStill(movement, { grows: ["heading", "head"] });
  await admin.dispose();
});
