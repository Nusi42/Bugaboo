import { createRouter } from "@tanstack/react-router";

import {
  appearanceRoute,
  framesRoute,
  homeRoute,
  invitationsRoute,
  mySettingsIndexRoute,
  organizationSettingsIndexRoute,
  securityRoute,
  sessionsRoute,
  shortcutsRoute,
  statusRoute,
} from "./app-pages.tsx";
import { invitationRoute, signUpRoute } from "./guest-pages.tsx";
import {
  adminRoute,
  appRoute,
  guestRoute,
  mySettingsRoute,
  organizationSettingsRoute,
  rootRoute,
} from "./layouts.tsx";
import { setupRoute } from "./setup.tsx";
import { secondStepRoute, signInRoute } from "./sign-in.tsx";

const routeTree = rootRoute.addChildren([
  guestRoute.addChildren([setupRoute, signInRoute, secondStepRoute, signUpRoute, invitationRoute]),
  appRoute.addChildren([
    homeRoute,
    framesRoute,
    mySettingsRoute.addChildren([
      mySettingsIndexRoute,
      appearanceRoute,
      shortcutsRoute,
      securityRoute,
      sessionsRoute,
    ]),
    adminRoute.addChildren([
      organizationSettingsRoute.addChildren([
        organizationSettingsIndexRoute,
        invitationsRoute,
        statusRoute,
      ]),
    ]),
  ]),
]);

/** The routes of the app, with the types of their addresses. */
export const router = createRouter({ routeTree, scrollRestoration: true });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
