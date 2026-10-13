import { createRoute, lazyRouteComponent } from "@tanstack/react-router";

import { paths, routeParam } from "../shared/paths.ts";
import { useAddressPart } from "./address-part.ts";
import { appRoute, organizationSettingsRoute } from "./layouts.tsx";
import { OnlyWith } from "./OnlyWith.tsx";
import { loadsScreen } from "./preload.ts";

// What roles see and what a person can do and why: three pages in *Organization settings* and the
// same three below the settings of a project. Loaded when first opened (coding-standards section 6).

const access = () => import("../features/access/index.ts");
const FieldPermissionsScreen = lazyRouteComponent(access, "FieldPermissionsPage");
const PeopleVisibilityScreen = lazyRouteComponent(access, "PeopleVisibilityPage");
const CheckAccessScreen = lazyRouteComponent(access, "CheckAccessPage");
const ProjectFieldsScreen = lazyRouteComponent(access, "ProjectFieldPermissionsScreen");
const ProjectPeopleScreen = lazyRouteComponent(access, "ProjectPeopleVisibilityScreen");
const ProjectCheckScreen = lazyRouteComponent(access, "ProjectCheckAccessScreen");

export const fieldPermissionsRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.fieldPermissions,
  loader: loadsScreen(FieldPermissionsScreen),
  component: function FieldPermissions() {
    return (
      <OnlyWith path={paths.fieldPermissions}>
        <FieldPermissionsScreen />
      </OnlyWith>
    );
  },
});

export const peopleVisibilityRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.peopleVisibility,
  loader: loadsScreen(PeopleVisibilityScreen),
  component: function PeopleVisibility() {
    return (
      <OnlyWith path={paths.peopleVisibility}>
        <PeopleVisibilityScreen />
      </OnlyWith>
    );
  },
});

export const checkAccessRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.checkAccess,
  loader: loadsScreen(CheckAccessScreen),
  component: function CheckAccess() {
    return (
      <OnlyWith path={paths.checkAccess}>
        <CheckAccessScreen />
      </OnlyWith>
    );
  },
});

/** `/p/WEB/settings/fields`: a project the person may not see is *Not found*, as everywhere. */
export const projectFieldsRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.projectFields,
  component: function ProjectFields() {
    const project = useAddressPart(routeParam.project);
    return project === undefined ? null : <ProjectFieldsScreen address={project} />;
  },
});

export const projectPeopleRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.projectPeople,
  component: function ProjectPeople() {
    const project = useAddressPart(routeParam.project);
    return project === undefined ? null : <ProjectPeopleScreen address={project} />;
  },
});

export const projectCheckAccessRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.projectCheckAccess,
  component: function ProjectCheckAccess() {
    const project = useAddressPart(routeParam.project);
    return project === undefined ? null : <ProjectCheckScreen address={project} />;
  },
});
