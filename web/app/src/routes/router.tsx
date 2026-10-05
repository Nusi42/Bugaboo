import { createRouter } from "@tanstack/react-router";

import {
  appearanceRoute,
  framesRoute,
  homeRoute,
  invitationsRoute,
  mySettingsIndexRoute,
  organizationSettingsIndexRoute,
  regionRoute,
  securityRoute,
  sessionsRoute,
  shortcutsRoute,
  signInRulesRoute,
  startPageRoute,
  statusRoute,
  themesRoute,
} from "./app-pages.tsx";
import { invitationRoute, signUpRoute } from "./guest-pages.tsx";
import {
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
      regionRoute,
      startPageRoute,
      shortcutsRoute,
      securityRoute,
      sessionsRoute,
    ]),
    organizationSettingsRoute.addChildren([
      organizationSettingsIndexRoute,
      invitationsRoute,
      signInRulesRoute,
      themesRoute,
      statusRoute,
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
