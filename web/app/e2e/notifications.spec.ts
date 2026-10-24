import { t } from "@bugaboo/messages";
import { ana, lea, roleId, roleName, setPeopleLevel, signedInAdmin } from "./support/access.ts";
import { bob } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { apiOf, signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { expectCountUnderTheBell } from "./support/menu-count.ts";
import {
  asBen,
  changeStatus,
  comment,
  expectUnread,
  follow,
  handleOf,
  menuEntry,
  openNotifications,
  readEverything,
  writeCommentOnThePage,
} from "./support/notifications.ts";
import { issueAddress, paths } from "../src/shared/paths.ts";

// Notifications on screen (NTF-01, M4 WP-08): what happened on issues a person follows and where
// they are named, the page with its filters, *Open* and *Mark as read*, and the start page. A
// comment is written in a second browser, the way another person writes it.

/** Stands where the old value would be in the sentence of a change, to cut the sentence there. */
const OLD_VALUE_HERE = "@@old-value-here@@";

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

test("ntf_01 a comment on an issue Ana follows raises her count, Open lands on it, Mark as read lowers it", async ({
  page,
  browser,
  server,
}) => {
  await signIn(page, ana);
  await follow(apiOf(page, server), "WEB-1");
  await expect(menuEntry(page)).toBeVisible();
  await expectUnread(page, 0);

  const ben = await asBen(browser, server);
  await writeCommentOnThePage(ben, "WEB-1", "The first note, from Ben.");
  await expectUnread(page, 1);
  // A screen with a list narrows the menu to its icons, where the number stands under the bell.
  await page.goto(issueAddress("WEB-1"));
  await expectCountUnderTheBell(page);

  await openNotifications(page);
  const group = page.getByRole("region", {
    name: t("notifications.group-name", { issue: "WEB-1" }),
  });
  await expect(group.getByText(t("notifications.commented", { who: bob.name }))).toBeVisible();
  await expect(group.getByText("The first note, from Ben.")).toBeVisible();

  // Open lands on the exact comment, which is marked.
  await group.getByRole("button", { name: new RegExp(`^${t("notifications.open")}`, "u") }).click();
  await expect(page).toHaveURL(/\/issues\/WEB-1\?comment=/u);
  const opened = page.locator('[data-part="history.entry"][data-state~="highlighted"]');
  await expect(opened).toContainText("The first note, from Ben.");
  await expect(opened).toBeInViewport();

  // Mark as read lowers the count.
  await openNotifications(page);
  await page
    .getByRole("button", { name: new RegExp(`^${t("notifications.mark-read")}`, "u") })
    .click();
  await expectUnread(page, 0);
  await expect(
    page
      .getByRole("region", { name: t("notifications.group-name", { issue: "WEB-1" }) })
      .getByText(t("notifications.new"), { exact: true }),
  ).toHaveCount(0);
});

test("ntf_01 Open of a change lands on the change", async ({ page, browser, server }) => {
  await signIn(page, ana);
  const ben = await asBen(browser, server);
  await changeStatus(apiOf(ben, server), "WEB-1", "done");
  await expectUnread(page, 1);
  await openNotifications(page);
  // The start of the sentence of the change, up to where the old value would stand.
  const sentence = t("history.changed", {
    who: bob.name,
    field: t("issue-field.status"),
    before: OLD_VALUE_HERE,
    after: "",
  });
  const sentenceStart = sentence.slice(0, sentence.indexOf(OLD_VALUE_HERE));
  await page
    .getByRole("button", { name: t("notifications.open-name", { what: sentenceStart }) })
    .click();
  await expect(page).toHaveURL(/\/issues\/WEB-1\?change=/u);
  await expect(
    page.locator('[data-part="history.entry"][data-state~="highlighted"]'),
  ).toBeVisible();
});

test("ntf_01 the filters show all, the unread ones and the mentions; the start page shows the newest unread", async ({
  page,
  browser,
  server,
}) => {
  await signIn(page, ana);
  await readEverything(apiOf(page, server));
  const benApi = apiOf(await asBen(browser, server), server);
  const handle = await handleOf(apiOf(page, server));
  await comment(benApi, "WEB-1", `Ana, can you check this? @${handle}`);
  await expectUnread(page, 1);

  // The start page: the newest unread notifications, in place of the old Mentions block.
  await page.goto(paths.home);
  const block = page.getByRole("region", { name: t("notifications.title"), exact: true });
  const mention = t("notifications.mentioned", { who: bob.name });
  await expect(block.getByText(mention)).toBeVisible();
  await expect(block.getByRole("link", { name: t("notifications.show-all") })).toBeVisible();

  await openNotifications(page);
  await page.getByRole("tab", { name: t("notifications.filter.mentions"), exact: true }).click();
  await expect(page.getByText(mention)).toBeVisible();
  await page.getByRole("tab", { name: t("notifications.filter.unread"), exact: true }).click();
  await expect(page.getByText(mention)).toBeVisible();
  await page
    .getByRole("button", { name: new RegExp(`^${t("notifications.mark-read")}`, "u") })
    .first()
    .click();
  await expect(page.getByText(t("notifications.nothing-unread"))).toBeVisible();
  await page.getByRole("tab", { name: t("notifications.filter.all"), exact: true }).click();
  await expect(page.getByText(mention)).toBeVisible();
});

test("ntf_01 the shortcut g n opens the notifications", async ({ page }) => {
  await signIn(page, ana);
  await page.keyboard.press("g");
  await page.keyboard.press("n");
  await expect(
    page.getByRole("heading", { level: 1, name: t("notifications.title"), exact: true }),
  ).toBeVisible();
});

test("ntf_01 a Guest sees Someone where Who sees people says so", async ({
  page,
  browser,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const guest = await roleId(admin, roleName.guest);
  await setPeopleLevel(admin, { role: guest, kind: "authors" }, "hidden");
  await admin.dispose();
  await signIn(page, lea);
  await follow(apiOf(page, server), "WEB-3");
  const benApi = apiOf(await asBen(browser, server), server);
  await comment(benApi, "WEB-3", "A note for the reporter.");
  await expectUnread(page, 1);
  await openNotifications(page);
  const someone = t("notifications.commented", { who: t("person.someone") });
  await expect(page.getByText(someone)).toBeVisible();
  await expect(page.getByText(bob.name)).toHaveCount(0);
});

test("ntf_01 the pages are good in every look", async ({ page }) => {
  await signIn(page, ana);
  await openNotifications(page);
  await expect(page.getByRole("region", { name: /WEB-1/u })).toBeVisible();
  await expectGoodInEveryLook(page);
  await page.goto(paths.home);
  await expect(
    page.getByRole("region", { name: t("notifications.title"), exact: true }),
  ).toBeVisible();
  await expectGoodInEveryLook(page);
});
