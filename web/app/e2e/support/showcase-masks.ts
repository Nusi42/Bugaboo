import { type Locator, type Page } from "@playwright/test";

// What the pictures of the showcase paint over (M4-F24): the few screens that show a moment the
// server made when the test ran (a date and time, how long the server runs), a secret (the link of
// an invitation) or the hash of the security log, which is new in every run. Everything else on
// these screens is in the picture, and every other screen is taken as it is.

/** A date as people write it, in any of the formats of the sample. */
const DATE = /\d{1,2}[./-]\d{1,2}[./-]\d{4}/u;
/** How long the server has been running. */
const RUNNING = /\d+ days?, \d+ hours?, \d+ minutes?/u;
/** The link of an invitation, with its secret after the hash sign. */
const INVITATION_LINK = /\/invitation#/u;
/** The sentence that says where the security log ends. */
const LOG_HASH = /hash starts with/u;

/** The screens that show what is new in every run, by their name in the file names. */
const SCREENS_WITH_NEWS: ReadonlySet<string> = new Set([
  "people",
  "invitations",
  "server-status",
  "sessions",
]);

/** The places of the screen that are painted over in its picture; none for most screens. */
export function maskOf(slug: string, page: Page): readonly Locator[] {
  return SCREENS_WITH_NEWS.has(slug)
    ? [
        page.getByText(DATE),
        page.getByText(RUNNING),
        page.getByText(INVITATION_LINK),
        page.getByText(LOG_HASH),
      ]
    : [];
}

/** The link of an invitation as the pictures show it: as long as every link, with a secret of x. */
const SHOWN_LINK = `https://localhost/invitation#${"x".repeat(43)}`;

/**
 * Makes what a mask cannot hide the same on every run: the pointer is out of the page, and the link of an invitation wraps at other
 * places for another secret, so the box around it is another height. A mask paints over a place
 * and leaves its size, so the text itself is written the same way every time.
 */
export async function settle(slug: string, page: Page): Promise<void> {
  // Where the pointer was left shows as a row under it, whichever row the page puts there.
  await page.mouse.move(0, 0);
  if (slug === "invitations") {
    await page.getByText(INVITATION_LINK).evaluate((link, text) => {
      link.textContent = text;
    }, SHOWN_LINK);
  }
}
