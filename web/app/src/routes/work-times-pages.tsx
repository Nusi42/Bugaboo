import { createRoute, lazyRouteComponent } from "@tanstack/react-router";

import { paths, routeParam } from "../shared/paths.ts";
import { useAddressPart } from "./address-part.ts";
import { appRoute, mySettingsRoute, organizationSettingsRoute } from "./layouts.tsx";
import { OnlyWith } from "./OnlyWith.tsx";
import { loadsScreen } from "./preload.ts";

// The screens of work times are loaded when they are first opened, so the first page stays small
// (coding-standards section 6).

const OwnWorkTimesScreen = lazyRouteComponent(
  () => import("../features/work-times/index.ts"),
  "OwnWorkTimesPage",
);
const PersonWorkTimesScreen = lazyRouteComponent(
  () => import("../features/work-times/index.ts"),
  "PersonWorkTimesPage",
);
const ProjectMembersScreen = lazyRouteComponent(
  () => import("../features/projects/index.ts"),
  "ProjectMembersPage",
);

/** *My settings › Work times*: usual work days, *Holidays*, away times and the shares (PLAN-11, PLAN-15). */
export const workTimesRoute = createRoute({
  getParentRoute: () => mySettingsRoute,
  path: paths.workTimes,
  component: OwnWorkTimesScreen,
});

/** The work times of one person, `/settings/people/<ID>/work-times`, for a holder of *Enter work times for others*. */
export const personWorkTimesRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.personWorkTimes,
  loader: loadsScreen(PersonWorkTimesScreen),
  component: function PersonWorkTimes() {
    const id = useAddressPart(routeParam.person);
    return (
      <OnlyWith path={paths.personWorkTimes}>
        {id === undefined ? null : <PersonWorkTimesScreen id={id} />}
      </OnlyWith>
    );
  },
});

/** *Project members* of a project, `/p/WEB/settings/members`: the people with their shares (PLAN-15). */
export const projectMembersRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.projectMembers,
  loader: loadsScreen(ProjectMembersScreen),
  component: function ProjectMembers() {
    const project = useAddressPart(routeParam.project);
    return project === undefined ? null : <ProjectMembersScreen address={project} />;
  },
});
