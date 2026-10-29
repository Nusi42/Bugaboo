import { issueAddress } from "./paths.ts";

/** What stands before the ID in the address of an issue: every page of an issue starts with it. */
const issuePrefix = issueAddress("");

/** The page the person was on before the issue page they are on now; none where they did not come from a page of the app. */
let before: string | undefined;

/** Whether an address is the page of an issue. */
function isIssue(pathname: string): boolean {
  return pathname.startsWith(issuePrefix);
}

/**
 * An address without the issue that is open in the side panel: *Back* from an issue must not
 * lead to a list that opens the same issue again.
 */
function withoutPanel(href: string): string {
  const place = new URL(href, "https://app.invalid");
  place.searchParams.delete("open");
  return `${place.pathname}${place.search}`;
}

/** A page of the app, as far as it is needed here. */
interface Place {
  readonly pathname: string;
  readonly href: string;
}

/**
 * Notes a change of page (what the router says when a page is resolved). The page that was left is
 * remembered while it is not an issue page, so moving from issue to issue does not lose where the
 * person came from.
 */
export function noteResolved(move: { readonly fromLocation?: Place | undefined }): void {
  const { fromLocation: from } = move;
  if (from !== undefined && !isIssue(from.pathname)) {
    before = withoutPanel(from.href);
  }
}

/** Where *Back to list* of an issue goes: the page the person came from, if they came from one. */
export function cameFrom(): string | undefined {
  return before;
}

/** Forgets the page the person came from: the next person at this browser has none. */
export function forgetCameFrom(): void {
  before = undefined;
}
