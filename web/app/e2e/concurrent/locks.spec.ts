import { t } from "@bugaboo/messages";

import { ada } from "../support/accounts.ts";
import { lostNote, savedNote, signedInPage } from "../support/crowd-pages.ts";
import { ben, cleo, crowdIssues, prepareCrowd } from "../support/crowd.ts";
import {
  descriptionBox,
  expectToldWhoIsEditing,
  isEditing,
  openEditing,
  saveButton,
  startEditing,
  typeUnderLock,
} from "../support/description.ts";
import { expect, test } from "../support/fixtures.ts";

// Ten people at the same time, the screens (architecture §11, M3 WP-10, TEST-CONCURRENT and
// TEST-EDIT-LOCKS): the people look at the pages, and the tests read what the pages tell them.

test.use({ preparation: { run: prepareCrowd } });
test.describe.configure({ mode: "serial" });

test("iss_17_three_people_edit_one_description_and_the_first_text_is_kept", async ({
  browser,
  server,
  viewport,
  hasTouch,
}) => {
  const look = { viewport, hasTouch };
  const holder = await signedInPage(browser, server, ben, look);
  const watcher = await signedInPage(browser, server, cleo, look);
  const taker = await signedInPage(browser, server, ada, look);
  await startEditing(holder, crowdIssues.description);
  await typeUnderLock(holder, "Ben's text, written first");
  // The other two are told who is editing and cannot type.
  await openEditing(watcher, crowdIssues.description);
  await openEditing(taker, crowdIssues.description);
  await Promise.all([watcher, taker].map((other) => expectToldWhoIsEditing(other, ben.name)));
  await expect(
    watcher.getByRole("button", { name: t("issues.ask-to-finish", { name: ben.name }) }),
  ).toBeVisible();
  // Edit anyway moves the lock to the one who may take it.
  await taker.getByRole("button", { name: t("issues.edit-anyway"), exact: true }).click();
  await descriptionBox(taker).fill("Ada's text, taken over");
  await saveButton(taker).click();
  await expect(savedNote(taker)).toBeVisible();
  await expect(taker.getByText("Ada's text, taken over").first()).toBeVisible();
  // Ben's time is over, but his text is still in his box, and he keeps it by comparing.
  await saveButton(holder).click();
  await expect(lostNote(holder)).toBeVisible();
  await expect(descriptionBox(holder)).toHaveValue("Ben's text, written first");
  await holder.getByRole("button", { name: t("issues.compare-and-save"), exact: true }).click();
  await expect(holder.getByText("Ada's text, taken over").first()).toBeVisible();
  await holder.getByRole("button", { name: t("issues.compare-save-mine"), exact: true }).click();
  await expect(holder.getByText("Ben's text, written first").first()).toBeVisible();
  // The third person, who only watched, reads the text that was kept.
  await watcher.reload();
  await expect(watcher.getByText("Ben's text, written first").first()).toBeVisible();
});

test("iss_17_a_lock_holders_network_drops_and_the_text_comes_back_with_compare_and_save", async ({
  browser,
  server,
  viewport,
  hasTouch,
  tolerate,
}) => {
  test.setTimeout(240_000);
  const look = { viewport, hasTouch };
  const holder = await signedInPage(browser, server, ben, look);
  const next = await signedInPage(browser, server, cleo, look);
  await startEditing(holder, crowdIssues.network);
  await typeUnderLock(holder, "Ben's words, typed before the network went");
  await openEditing(next, crowdIssues.network);
  await expect(isEditing(next, ben.name)).toBeVisible();
  // The network of the holder drops: the requests that fail are what the browser writes about it.
  tolerate(/ERR_INTERNET_DISCONNECTED|WebKit encountered an internal error/u);
  await holder.context().setOffline(true);
  // Within a minute (the lease) and the time the box takes to notice, the lock is free and the
  // box opens for the next person, who writes and saves.
  await expect(descriptionBox(next)).toBeVisible({ timeout: 100_000 });
  await descriptionBox(next).fill("Cleo's text, written while Ben was away");
  await saveButton(next).click();
  await expect(savedNote(next)).toBeVisible();
  // The network is back: Ben's text is still in his box and he can compare and save it.
  await holder.context().setOffline(false);
  await saveButton(holder).click();
  await expect(lostNote(holder)).toBeVisible();
  await expect(descriptionBox(holder)).toHaveValue("Ben's words, typed before the network went");
  await holder.getByRole("button", { name: t("issues.compare-and-save"), exact: true }).click();
  await expect(holder.getByText("Cleo's text, written while Ben was away").first()).toBeVisible();
  await holder.getByRole("button", { name: t("issues.compare-save-mine"), exact: true }).click();
  await expect(
    holder.getByText("Ben's words, typed before the network went").first(),
  ).toBeVisible();
});
