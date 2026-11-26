/**
 * The values of `aria-current`, words of the ARIA standard and not text for people, written once: the page of a menu,
 * a tree or a trail that is open (`page`), and the one chosen of a set, such as the column of the chosen role or the
 * open item of a list of buttons (`chosen`, which ARIA writes `true`), and today on a calendar (`date`). Components and browser tests read them here.
 */
export const ariaCurrent = { page: "page", chosen: "true", date: "date" } as const;
