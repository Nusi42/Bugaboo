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
import {
  fieldRoute,
  fieldsRoute,
  issueRoute,
  issueTypesRoute,
  labelsRoute,
  layoutsRoute,
  projectIssuesRoute,
  projectIssueTypesRoute,
  projectLabelsRoute,
  projectLayoutsRoute,
  projectWorkflowsRoute,
  relationKindsRoute,
  savedSearchRoute,
  searchRoute,
  workflowsRoute,
} from "./work-pages.tsx";
import {
  accessKeysRoute,
  appearanceRoute,
  handleRoute,
  homeRoute,
  mailServerRoute,
  notificationSettingsRoute,
  notificationsRoute,
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
    notificationsRoute,
    searchRoute,
    savedSearchRoute,
    projectRoute,
    projectIssuesRoute,
    issueRoute,
    projectSettingsRoute,
    projectRolesRoute,
    projectFieldsRoute,
    projectLayoutsRoute,
    projectIssueTypesRoute,
    projectLabelsRoute,
    projectWorkflowsRoute,
    projectPeopleRoute,
    projectCheckAccessRoute,
    mySettingsRoute.addChildren([
      mySettingsIndexRoute,
      appearanceRoute,
      regionRoute,
      startPageRoute,
      notificationSettingsRoute,
      handleRoute,
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
      relationKindsRoute,
      labelsRoute,
      layoutsRoute,
      workflowsRoute,
      fieldsRoute,
      fieldRoute,
      fieldPermissionsRoute,
      peopleVisibilityRoute,
      checkAccessRoute,
      signInRulesRoute,
      themesRoute,
      projectsRoute,
      mailServerRoute,
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
