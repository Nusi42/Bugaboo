/** The id the "skip to the content" link jumps to. */
export const CONTENT_ID = "content";
export const CONTENT_HREF = `#${CONTENT_ID}`;

/**
 * Tells the router which element scrolls, so that going back brings the page to where it was (a
 * phone scrolls the content, not the window). The name is the attribute of TanStack Router.
 */
export const scrollRestoration = { "data-scroll-restoration-id": CONTENT_ID } as const;
