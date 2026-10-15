import {
  descriptionEditPart,
  heartbeatSeconds,
  leaseSeconds,
  readDraft,
} from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { ada, type Person, signInThroughApi } from "../support/accounts.ts";
import { signedInAdmin } from "../support/access.ts";
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
import { type Instance } from "../support/server.ts";
import { issueOf } from "../support/issues.ts";

// Ten people at the same time, the screens (architecture §11, M3 WP-10, TEST-CONCURRENT and
// TEST-EDIT-LOCKS): the people look at the pages, and the tests read what the pages tell them.

test.use({ preparation: { run: prepareCrowd } });
test.describe.configure({ mode: "serial" });

/**
 * How long a page takes to notice that a lock is gone: it looks again at every heartbeat, and a
 * few seconds are left for the answer. The lock itself is free within the lease.
 */
const noticeSeconds = heartbeatSeconds + 5;

/**
 * The whole line a browser writes for a request that fails because its network is cut: Chromium
 * names the error, WebKit can only say that it had an internal error (which is all it says).
 */
const cutNetwork =
  /^Failed to load resource: (?:net::ERR_INTERNET_DISCONNECTED|WebKit encountered an internal error)$/u;

/** The draft of the description of an issue that the person has on the server, read as that person. */
async function draftOf(server: Instance, person: Person, key: string) {
  const session = await signInThroughApi(server, person);
  const admin = await signedInAdmin(server);
  try {
    const issue = await issueOf(admin, key);
    return await readDraft(session.client, {
      kind: "issue",
      id: issue.id,
      part: descriptionEditPart,
    });
  } finally {
    await Promise.all([session.dispose(), admin.dispose()]);
  }
}

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
  // The server has Ben's text too: it is his draft, whatever happens to his page.
  await expect
    .poll(async () => (await draftOf(server, ben, crowdIssues.description))?.text)
    .toBe("Ben's text, written first");
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
  // The lock frees within the lease and the page notices within a few seconds more: the time the
  // test may take is the one of the configuration plus the time it waits for that.
  test.setTimeout(test.info().timeout + (leaseSeconds + noticeSeconds) * 1000);
  const look = { viewport, hasTouch };
  const holder = await signedInPage(browser, server, ben, look);
  const next = await signedInPage(browser, server, cleo, look);
  await startEditing(holder, crowdIssues.network);
  await typeUnderLock(holder, "Ben's words, typed before the network went");
  await openEditing(next, crowdIssues.network);
  await expect(isEditing(next, ben.name)).toBeVisible();
  // The network of the holder drops: the requests of that browser that fail are what it writes
  // about it, and only while it is offline.
  const network = tolerate(holder.context(), cutNetwork);
  await network.begin();
  // Within a minute (the lease) and the time the box takes to notice, the lock is free and the
  // box opens for the next person, who writes and saves.
  await expect(descriptionBox(next)).toBeVisible({
    timeout: (leaseSeconds + noticeSeconds) * 1000,
  });
  await descriptionBox(next).fill("Cleo's text, written while Ben was away");
  await saveButton(next).click();
  await expect(savedNote(next)).toBeVisible();
  // The network is back: Ben's text is still in his box and he can compare and save it.
  await network.end();
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
