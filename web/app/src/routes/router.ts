import { createRouter } from "@tanstack/react-router";

import { boxesToTop } from "../shared/scroll-boxes.ts";
import { routeTree } from "./route-tree.tsx";

/**
 * The routes of the app, with the types of their addresses. A new page starts its boxes at the top (`boxesToTop`),
 * and Back brings each to where it was (UX §6.5).
 */
export const router = createRouter({
  routeTree,
  scrollRestoration: true,
  scrollToTopSelectors: [...boxesToTop],
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
