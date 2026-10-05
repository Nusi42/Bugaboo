import { createRouter } from "@tanstack/react-router";

import {
  homeRoute,
  invitationsRoute,
  securityRoute,
  sessionsRoute,
  statusRoute,
} from "./app-pages.tsx";
import { invitationRoute, signUpRoute } from "./guest-pages.tsx";
import { adminRoute, appRoute, guestRoute, rootRoute } from "./layouts.tsx";
import { setupRoute } from "./setup.tsx";
import { secondStepRoute, signInRoute } from "./sign-in.tsx";

const routeTree = rootRoute.addChildren([
  guestRoute.addChildren([setupRoute, signInRoute, secondStepRoute, signUpRoute, invitationRoute]),
  appRoute.addChildren([
    homeRoute,
    securityRoute,
    sessionsRoute,
    adminRoute.addChildren([invitationsRoute, statusRoute]),
  ]),
]);

/** The routes of the app, with the types of their addresses. */
export const router = createRouter({ routeTree, scrollRestoration: true });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
