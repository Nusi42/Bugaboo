import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { ada, bob } from "./support/accounts.ts";
import { expect, refusal, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { recordRequests } from "./support/list-keys.ts";
import { expectStill, landmarkOf, movementWhile } from "./support/movement.ts";
import { adminWithOldAndNewTime, preparePrivacy } from "./support/privacy.ts";

// *My data*, *Data exports* and *Keep and delete rules* (PRIV-03, PRIV-05; UX §4.13, §5.7; ADR-0044, ADR-0045).

test.use({ preparation: { run: preparePrivacy } });

/** The job writes the file of a person's data in the background: seconds on a quiet computer. */
const FILE_CREATED_MS = 60_000;

test("priv_03 a person downloads their data from My data and the page does not move while it is made", async ({
  page,
}) => {
  await signIn(page, bob);
  await page.goto(paths.myData);
  const download = page.getByRole("region", { name: t("my-data.download"), exact: true });
  const correct = page.getByRole("region", { name: t("my-data.correct-title"), exact: true });
  await expect(correct).toBeVisible();
  // The part of the file stands, in the size of all its states, before the button is pressed.
  await expect(download.getByText(t("my-data.none"))).toBeVisible();
  const landmarks = [
    await landmarkOf(page.getByRole("heading", { level: 1 }), "heading"),
    await landmarkOf(download, "download"),
    await landmarkOf(
      page.getByRole("region", { name: t("keep-group.name.by-itself"), exact: true }),
      "rules",
    ),
    await landmarkOf(correct, "correct"),
  ];
  const movement = await movementWhile(page, landmarks, async () => {
    await download.getByRole("button", { name: t("my-data.download"), exact: true }).click();
    // While the file is created the button says so, in its place.
    await expect(download.getByRole("button", { name: t("my-data.making") })).toBeVisible();
    await expect(
      download.getByRole("button", { name: t("my-data.download-file"), exact: true }),
    ).toBeVisible({ timeout: FILE_CREATED_MS });
  });
  // The file is made and ready in the room the part had from the start: nothing grows or moves.
  expectStill(movement);
  await expect(
    download.getByRole("button", { name: t("my-data.download"), exact: true }),
  ).toBeEnabled();
  // A file the server no longer gives out is said in place, in plain words.
  await page.route(/\/api\/v1\/data-exports\/[^/]+\/address$/u, (route) =>
    route.fulfill(refusal(404, "resource.not-found")),
  );
  await download.getByRole("button", { name: t("my-data.download-file"), exact: true }).click();
  await expect(download.getByText(t("error.resource.not-found"))).toBeVisible();
});

test("priv_03 an administrator makes someone's export and it is listed in Data exports", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(paths.dataExports);
  // How long a file is kept comes from the keep rules.
  await expect(
    page.getByText(t("data-exports.intro", { period: t("keep-length.days", { count: 7 }) })),
  ).toBeVisible();
  await page.getByRole("button", { name: t("data-exports.someone"), exact: true }).click();
  const window = page.getByRole("dialog", { name: t("data-exports.someone") });
  await window.getByRole("searchbox", { name: t("data-exports.pick-search") }).fill("Bob");
  await window.getByRole("radio", { name: bob.name }).check();
  const reason = "Bob asked for his data by e-mail";
  await window.getByRole("textbox", { name: t("data-exports.reason") }).fill(reason);
  await window.getByRole("button", { name: t("data-exports.create"), exact: true }).click();
  await expect(window).toBeHidden();
  const row = page
    .getByRole("table", { name: t("data-exports.list") })
    .getByRole("row")
    .filter({ hasText: reason });
  await expect(row.getByRole("rowheader")).toHaveText(bob.name);
  await expect(row).toContainText(ada.name);
});

test("priv_05 the keep table shows both groups and a changed period says what it deletes first", async ({
  page,
  server,
}) => {
  const admin = await adminWithOldAndNewTime(server);
  // What a changed period would delete is counted for a changed period only, once the person stops typing.
  const asked = recordRequests(page);
  const counted = (): readonly string[] => asked.filter((url) => url.includes("/would-delete"));
  await signIn(page, ada);
  await page.goto(paths.keepRules);
  const itself = page.getByRole("table", { name: t("keep-group.name.by-itself") });
  const administrator = page.getByRole("table", {
    name: t("keep-group.name.by-an-administrator"),
  });
  await expect(
    itself.getByRole("rowheader", { name: t("keep-kind.name.notifications") }),
  ).toBeVisible();
  const timeRow = administrator
    .getByRole("row")
    .filter({ has: page.getByRole("rowheader", { name: t("keep-kind.name.time-entries") }) });
  await expect(timeRow).toContainText(t("keep-length.years", { count: 10 }));
  // A shorter period says what it would leave past it before anything is saved.
  const years = page.getByRole("textbox", { name: t("setting.keeping.time-entry-years.name") });
  await expect(years).toHaveValue("10");
  expect(counted()).toEqual([]);
  await years.pressSequentially("05");
  await years.fill("5");
  const past = t("keep-rules.would-be-past", {
    records: t("keep-kind.count.time-entries", { count: 1 }),
    count: 1,
  });
  await expect(page.getByText(past)).toBeVisible();
  // Typed key by key, only the number the person stopped at was counted.
  expect(counted()).toHaveLength(1);
  expect(counted()[0]).toContain("count=5");
  await expect(timeRow).toContainText(t("keep-length.years", { count: 10 }));
  // Saved, the table shows the new period.
  await years.press("Enter");
  await expect(timeRow).toContainText(t("keep-length.years", { count: 5 }));
  await years.fill("10");
  await years.press("Enter");
  await expect(timeRow).toContainText(t("keep-length.years", { count: 10 }));
  await admin.dispose();
});

test("priv_05 Delete them deletes only what is past its period and says how many", async ({
  page,
  server,
}) => {
  const admin = await adminWithOldAndNewTime(server);
  await signIn(page, ada);
  await page.goto(paths.keepRules);
  const card = page.getByRole("region", {
    name: t("old-records.title", { kind: t("keep-kind.name.time-entries") }),
  });
  await expect(
    card.getByText(/^\d+ time entr(y is|ies are) older than the keep period/u),
  ).toBeVisible();
  await card.getByRole("button", { name: t("old-records.delete-all"), exact: true }).click();
  await page
    .getByRole("alertdialog")
    .getByRole("button", { name: t("old-records.delete"), exact: true })
    .click();
  await expect(card.getByText(/time entr(y was|ies were) deleted\.$/u)).toBeVisible();
  await expect(card.getByText(t("old-records.none"))).toBeVisible();
  // The entry of today is still there; only those of 2014 went.
  const today = new Date().toISOString().slice(0, 10);
  const kept = await admin.client.GET("/api/v1/me/time-entries", {
    params: { query: { from: today, to: today } },
  });
  expect(dataOf(kept).items).not.toHaveLength(0);
  await admin.dispose();
});
