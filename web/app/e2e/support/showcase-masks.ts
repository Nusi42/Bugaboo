import { t } from "@bugaboo/messages";
import { type Locator, type Page } from "@playwright/test";

// What the pictures of the showcase paint over (M4-F24): the few screens that show a moment the
// server made when the test ran (a date and time, how long the server runs), a secret (the link of
// an invitation) or the hash of the security log, which is new in every run. Everything else on
// these screens is in the picture, and every other screen is taken as it is.

/** A date as people write it, in any of the formats of the sample. */
const DATE = /\d{1,2}[./-]\d{1,2}[./-]\d{4}/u;

/** What stands for a part of a text that changes (a hash, a number) while the words round it do not. */
const CHANGING = "CHANGING";

/** The characters of a text that mean something in a pattern. */
const PATTERN_SYMBOLS = /[.*+?^${}()|[\]\\]/gu;

/** The words of the catalog that name a count, and the numbers: they change, the rest does not. */
const COUNTS = /\d+|days|hours|minutes/gu;

/**
 * A pattern for a text of the catalog as it was written with sample values: the words are the
 * catalog's, a number is any number (and the unit after it may be one or many), and `CHANGING` is
 * anything.
 */
function textLike(text: string): RegExp {
  const written = text
    .replaceAll(PATTERN_SYMBOLS, String.raw`\$&`)
    .replaceAll(CHANGING, ".+?")
    .replaceAll(COUNTS, (found) =>
      /\d/u.test(found) ? String.raw`\d+` : `${found.slice(0, -1)}s?`,
    );
  return new RegExp(written, "u");
}

/** How long the server has been running, in the words of the page. */
const RUNNING = textLike(t("server-status.uptime", { days: 2, hours: 2, minutes: 2 }));
/** The link of an invitation, with its secret after the hash sign. */
const INVITATION_LINK = /\/invitation#/u;
/** The sentence that says where the security log ends, with its number and hash. */
const LOG_HASH = textLike(t("security-log.head", { seq: CHANGING, hash: CHANGING }));

/** The screens that show what is new in every run, by their name in the file names. */
const SCREENS_WITH_NEWS: ReadonlySet<string> = new Set([
  "organization-settings",
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
 * Makes what a mask cannot hide the same on every run: the pointer is out of the page, and the link
 * of an invitation wraps at other places for another secret, so the box around it is another
 * height. A mask paints over a place and leaves its size, so the text itself is written the same
 * way every time.
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
