import { createRouter } from "@tanstack/react-router";

import {
  groupsRoute,
  peopleRoute,
  personRoute,
  projectRolesRoute,
  roleRoute,
  rolesRoute,
} from "./access-pages.tsx";
import {
  checkAccessRoute,
  fieldPermissionsRoute,
  peopleVisibilityRoute,
  projectCheckAccessRoute,
  projectFieldsRoute,
  projectPeopleRoute,
} from "./what-roles-see-pages.tsx";
import { fieldRoute, fieldsRoute, issueTypesRoute } from "./work-pages.tsx";
import {
  accessKeysRoute,
  appearanceRoute,
  framesRoute,
  homeRoute,
  mySettingsIndexRoute,
  organizationSecurityRoute,
  organizationSettingsIndexRoute,
  regionRoute,
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
import { projectRoute, projectSettingsRoute } from "./project-pages.tsx";
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
    projectRolesRoute,
    projectFieldsRoute,
    projectPeopleRoute,
    projectCheckAccessRoute,
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
      peopleRoute,
      personRoute,
      groupsRoute,
      rolesRoute,
      roleRoute,
      issueTypesRoute,
      fieldsRoute,
      fieldRoute,
      fieldPermissionsRoute,
      peopleVisibilityRoute,
      checkAccessRoute,
      signInRulesRoute,
      themesRoute,
      projectsRoute,
      organizationSecurityRoute,
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
