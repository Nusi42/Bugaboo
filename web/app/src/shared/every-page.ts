/** What a list read page by page says of its reading. */
export interface PagesRead {
  readonly isLoading: boolean;
  readonly hasNextPage: boolean;
  readonly isError: boolean;
}

/**
 * Whether a list that is read page by page, every page of it, is still being read for the first time: its first page
 * is on its way, or more pages are to come. A page that could not be read ends it, so what waits for the list (a hold
 * of its screen) shows and says why rather than waiting for ever (UX §6.4).
 */
export function readingEveryPage(read: PagesRead): boolean {
  return !read.isError && (read.isLoading || read.hasNextPage);
}
