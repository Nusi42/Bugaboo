/** Whether an address is the page at `path` or a page under it (`/settings/people/<ID>` under `/settings/people`). */
export function isAtOrUnder(path: string, pathname: string): boolean {
  return pathname === path || pathname.startsWith(`${path}/`);
}

/**
 * The topic of a menu that holds the open page: the deepest one the address is at or under, so the page of a person
 * marks *People* and a page of the settings of a project marks its own topic, not the first one. None where no topic
 * holds the page, or no page is open (the menu is a screen of its own).
 */
export function deepestAround(topics: readonly string[], pathname?: string): string | undefined {
  let deepest: string | undefined;
  if (pathname !== undefined) {
    for (const topic of topics) {
      if (isAtOrUnder(topic, pathname) && topic.length > (deepest?.length ?? -1)) {
        deepest = topic;
      }
    }
  }
  return deepest;
}
