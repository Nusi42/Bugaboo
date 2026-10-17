import { t } from "@bugaboo/messages";
import { type Browser, type BrowserContext, expect, type Page } from "@playwright/test";

import { type Person, signInThroughApi } from "./accounts.ts";
import { saveButton } from "./description.ts";
import { crowd, crowdSize, type Edit } from "./crowd.ts";
import { openIssueWithDetails } from "./flows.ts";
import { changeField, savedNote, statusButton } from "./issues.ts";
import { type Instance } from "./server.ts";

// The pages of ten people at the same time (architecture §11, M3 WP-10).

/** One try of a step that may find the page in either of two states, and the time all tries get. */
const RETRY_MS = 2000;
const SETTLE_MS = 20_000;

/** How the window of a person looks: the part of the project's setup that a context of its own does not inherit. */
export interface ScreenLook {
  readonly viewport: { readonly width: number; readonly height: number } | null;
  readonly hasTouch: boolean;
}

/** A browser of its own, signed in as the person without typing a password (the sign-in is tested elsewhere). */
export async function signedInBrowser(
  browser: Browser,
  server: Instance,
  person: Person,
  look: ScreenLook,
): Promise<BrowserContext> {
  const session = await signInThroughApi(server, person);
  const context = await browser.newContext({
    baseURL: server.url,
    ignoreHTTPSErrors: true,
    storageState: await session.state(),
    hasTouch: look.hasTouch,
    ...(look.viewport === null ? {} : { viewport: look.viewport }),
  });
  return context;
}

/** A page of a browser of its own, signed in as the person. */
export async function signedInPage(
  browser: Browser,
  server: Instance,
  person: Person,
  look: ScreenLook,
): Promise<Page> {
  return (await signedInBrowser(browser, server, person, look)).newPage();
}

/** One page for each of the first `count` people of the crowd, each in a browser of its own. */
export async function openCrowd(
  browser: Browser,
  server: Instance,
  look: ScreenLook,
  count: number = crowdSize,
): Promise<readonly Page[]> {
  const contexts = await Promise.all(
    crowd.slice(0, count).map((person) => signedInBrowser(browser, server, person, look)),
  );
  return Promise.all(contexts.map((context) => context.newPage()));
}

/** Closes the browsers of the pages. */
export async function closeCrowd(pages: readonly Page[]): Promise<void> {
  await Promise.all(pages.map((page) => page.context().close()));
}

/** The note of a change that came second (*Ben changed Priority to Critical …*), with its two answers. */
export function conflictNote(page: Page) {
  return page.getByRole("button", { name: t("issues.use-mine"), exact: true });
}

/**
 * Waits until the change of the page is kept, the way a person does: they look at the page, and if
 * it says somebody changed the issue first they press *Use mine* and look again. Nothing is
 * assumed about how long the server takes. Returns how often *Use mine* was pressed.
 */
export async function untilKept(page: Page, pressed = 0): Promise<number> {
  const second = conflictNote(page);
  await expect(savedNote(page).or(second)).toBeVisible();
  if (!(await second.isVisible())) {
    return pressed;
  }
  // The change goes out again; what the page says next is read after the server has answered.
  const answered = page.waitForResponse(
    (answer) => answer.request().method() === "PATCH" && answer.url().includes("/issues/"),
  );
  await second.click();
  await answered;
  return untilKept(page, pressed + 1);
}

/** The note that says the text was not saved and is kept (*Not saved yet …*). */
export function lostEditNotice(page: Page) {
  return page.getByText(new RegExp(`^${t("save.not-saved")}`, "u")).first();
}

/**
 * Saves a text whose editing somebody took over, and waits for the note that says it was not saved.
 * The page of the one who lost the lock is told on its next heartbeat (every few seconds) or when
 * it saves, whichever comes first: when the heartbeat came first, *Save* is off already and the
 * note is there, which is what the test is about.
 */
export async function saveAfterTakeOver(page: Page): Promise<void> {
  const save = saveButton(page);
  const note = lostEditNotice(page);
  await expect(async () => {
    if (await save.isEnabled()) {
      await save.click({ timeout: RETRY_MS });
    }
    await expect(note).toBeVisible({ timeout: RETRY_MS });
  }).toPass({ timeout: SETTLE_MS });
}

/** Opens an issue with its details and waits until the page shows the status. */
export async function openIssueAndWait(page: Page, key: string): Promise<void> {
  await openIssueWithDetails(page, key);
  await expect(statusButton(page)).toBeVisible();
}

/** Who was told that somebody else was first: of two changes made together, one is kept and the other is not. */
export async function whoWasTold(
  first: Page,
  second: Page,
): Promise<{ readonly lost: Page; readonly won: Page }> {
  await expect(savedNote(first).or(conflictNote(first))).toBeVisible();
  await expect(savedNote(second).or(conflictNote(second))).toBeVisible();
  const firstLost = await conflictNote(first).isVisible();
  return firstLost ? { lost: first, won: second } : { lost: second, won: first };
}

/**
 * Opens the issue for a person and gives back what that person does next: change their field and
 * wait until it is kept, pressing *Use mine* as needed. The opening and the changing are apart so
 * that ten people can all open first and then all change together.
 */
export async function readyToChange(
  browser: Browser,
  server: Instance,
  look: ScreenLook,
  edit: Edit,
  key: string,
): Promise<() => Promise<void>> {
  const page = await signedInPage(browser, server, edit.person, look);
  await openIssueAndWait(page, key);
  return async () => {
    await changeField(page, edit.label, edit.choice);
    await untilKept(page);
  };
}
