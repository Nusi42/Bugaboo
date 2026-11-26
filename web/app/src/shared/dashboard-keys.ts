/**
 * The keys of what the dashboards keep in the query cache. They are part of `queryKeys.dashboards` (`query-keys.ts`);
 * they are written here only because that file is as long as a file may be.
 */
export const dashboardKeys = {
  all: ["dashboards"],
  /** Every list of dashboards (the names in the sidebar). */
  lists: () => ["dashboards", "list"],
  list: (which: string, size: number) => ["dashboards", "list", which, size],
  /** What every box of one dashboard answers. */
  boxesOf: (dashboard: string) => ["dashboards", "box", dashboard],
  one: (id: string) => ["dashboards", "one", id],
  home: () => ["dashboards", "home"],
  /** What one box answers, for the version of its dashboard that placed it. */
  box: (dashboard: string, box: string) => ["dashboards", "box", dashboard, box],
  readyMade: () => ["dashboards", "ready-made"],
  /** The preview of a box that is not on a dashboard yet, by what it asks. */
  preview: (asked: string) => ["dashboards", "preview", asked],
  shareChoices: (kind: string) => ["dashboards", "share-choices", kind],
};
