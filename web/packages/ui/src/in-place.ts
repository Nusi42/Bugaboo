/**
 * How the app moves the focus itself: without scrolling to it (UX §6.4, nothing moves under the
 * cursor). A heading or a page that takes the focus is where the person looks already, and the
 * browser would otherwise scroll the page by the height of what stands above it, then the router
 * scrolls back: the page jumped for a frame.
 */
export const inPlace: FocusOptions = { preventScroll: true };
