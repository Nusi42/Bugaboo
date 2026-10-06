import { createRouter } from "@tanstack/react-router";

import {
  accessKeysRoute,
  appearanceRoute,
  framesRoute,
  homeRoute,
  invitationsRoute,
  mySettingsIndexRoute,
  organizationSettingsIndexRoute,
  regionRoute,
  projectRoute,
  projectSettingsRoute,
  projectsRoute,
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
  setUpRoute,
} from "./layouts.tsx";
import { setupRoute } from "./setup.tsx";
import { secondStepRoute, setUpSecondStepRoute, signInRoute } from "./sign-in.tsx";

const routeTree = rootRoute.addChildren([
  guestRoute.addChildren([setupRoute, signInRoute, secondStepRoute, signUpRoute, invitationRoute]),
  setUpRoute.addChildren([setUpSecondStepRoute]),
  appRoute.addChildren([
    homeRoute,
    framesRoute,
    projectRoute,
    projectSettingsRoute,
    mySettingsRoute.addChildren([
      mySettingsIndexRoute,
      appearanceRoute,
      regionRoute,
      startPageRoute,
      shortcutsRoute,
      securityRoute,
      sessionsRoute,
      accessKeysRoute,
    ]),
    organizationSettingsRoute.addChildren([
      organizationSettingsIndexRoute,
      invitationsRoute,
      signInRulesRoute,
      themesRoute,
      projectsRoute,
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
