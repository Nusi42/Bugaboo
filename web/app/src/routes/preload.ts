/**
 * The loader of a route whose screen is loaded when it is first opened (`lazyRouteComponent`) but
 * is not the route's own `component`, because `OnlyWith` stands between them. The router waits for
 * a loader before it shows the page, so the screen has its code (and its message part) when it
 * is drawn; without the loader the page would be drawn empty first, and the focus would move to
 * a heading that is not there yet.
 */
export function loadsScreen(screen: {
  readonly preload?: (() => Promise<void> | undefined) | undefined;
}): () => Promise<void> {
  return async () => {
    await screen.preload?.();
  };
}
