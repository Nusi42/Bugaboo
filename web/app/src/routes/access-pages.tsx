import { createRoute, lazyRouteComponent, useNavigate, useSearch } from "@tanstack/react-router";

import { paths, routeParam } from "../shared/paths.ts";
import { useAddressPart } from "./address-part.ts";
import { appRoute, organizationSettingsRoute } from "./layouts.tsx";
import { OnlyWith } from "./OnlyWith.tsx";

// The screens of people, groups and roles are loaded when they are first opened, so the first page
// stays small (coding-standards section 6).

const ProjectRolesScreen = lazyRouteComponent(
  () => import("../features/projects/index.ts"),
  "ProjectRolesPage",
);
const PeopleScreen = lazyRouteComponent(() => import("../features/people/index.ts"), "PeoplePage");
const PersonScreen = lazyRouteComponent(() => import("../features/people/index.ts"), "PersonPage");
const GroupsScreen = lazyRouteComponent(() => import("../features/people/index.ts"), "GroupsPage");
const RolesScreen = lazyRouteComponent(() => import("../features/roles/index.ts"), "RolesPage");
const RoleScreen = lazyRouteComponent(() => import("../features/roles/index.ts"), "RolePage");

/** *Organization settings › People*: everybody, the invitations, and the rule for signing up. */
export const peopleRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.people,
  component: function People() {
    return (
      <OnlyWith path={paths.people}>
        <PeopleScreen />
      </OnlyWith>
    );
  },
});

/** The page of one person, `/settings/people/<ID>`. */
export const personRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.person,
  component: function Person() {
    const id = useAddressPart(routeParam.person);
    return (
      <OnlyWith path={paths.person}>{id === undefined ? null : <PersonScreen id={id} />}</OnlyWith>
    );
  },
});

/** *Organization settings › Groups*: the address says which group is open in the side panel. */
export const groupsRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.groups,
  validateSearch: (search: Readonly<Record<string, unknown>>): { open?: string | undefined } => ({
    open: typeof search["open"] === "string" ? search["open"] : undefined,
  }),
  component: function Groups() {
    const { open } = useSearch({ strict: false });
    const navigate = useNavigate();
    return (
      <OnlyWith path={paths.groups}>
        <GroupsScreen
          open={open}
          onOpen={(id: string) => {
            void navigate({ to: paths.groups, search: { open: id } });
          }}
          onClose={() => {
            void navigate({ to: paths.groups, search: {} });
          }}
        />
      </OnlyWith>
    );
  },
});

/** *Organization settings › Roles & permissions*: the roles and *Who has which role*. */
export const rolesRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.roles,
  component: function Roles() {
    return (
      <OnlyWith path={paths.roles}>
        <RolesScreen />
      </OnlyWith>
    );
  },
});

/** The page of one role, `/settings/roles/<ID>`. */
export const roleRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.role,
  component: function Role() {
    const id = useAddressPart(routeParam.role);
    return (
      <OnlyWith path={paths.role}>{id === undefined ? null : <RoleScreen id={id} />}</OnlyWith>
    );
  },
});

/** *Who has which role* in a project, `/p/WEB/settings/roles`. */
export const projectRolesRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.projectRoles,
  component: function ProjectRoles() {
    const project = useAddressPart(routeParam.project);
    return project === undefined ? null : <ProjectRolesScreen address={project} />;
  },
});
