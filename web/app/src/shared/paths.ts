/**
 * The addresses of the pages of the app, written once. Routes, links and redirects all use these,
 * so an address cannot differ between the page and the link to it (ODR). They are the addresses of
 * the UX design (§2.1): `/me/…` for *My settings*, `/settings/…` for *Organization settings*.
 */
export const paths = {
  home: "/",
  setup: "/setup",
  signIn: "/sign-in",
  secondStep: "/sign-in/second-step",
  setUpSecondStep: "/sign-in/set-up-second-step",
  signUp: "/sign-up",
  invitation: "/invitation",
  mySettings: "/me",
  appearance: "/me/appearance",
  region: "/me/region",
  startPage: "/me/start-page",
  shortcuts: "/me/shortcuts",
  security: "/me/security",
  sessions: "/me/sessions",
  accessKeys: "/me/access-keys",
  organizationSettings: "/settings",
  people: "/settings/people",
  person: "/settings/people/$person",
  groups: "/settings/groups",
  roles: "/settings/roles",
  role: "/settings/roles/$role",
  signInRules: "/settings/sign-in-rules",
  themes: "/settings/themes",
  status: "/settings/status",
  projects: "/settings/projects",
  fieldPermissions: "/settings/field-permissions",
  peopleVisibility: "/settings/people-visibility",
  checkAccess: "/settings/check-access",
  project: "/p/$project",
  projectSettings: "/p/$project/settings",
  projectRoles: "/p/$project/settings/roles",
  projectFields: "/p/$project/settings/fields",
  projectPeople: "/p/$project/settings/people",
  projectCheckAccess: "/p/$project/settings/check-access",
  frames: "/frames",
} as const;

/** An address of the app, as the routes know it. */
export type Path = (typeof paths)[keyof typeof paths];

/** The parts of an address that change from page to page, by the name the route gives them. */
export const routeParam = { project: "project", person: "person", role: "role" } as const;

/** The name of a part of an address that changes. */
export type RouteParam = (typeof routeParam)[keyof typeof routeParam];

/** The address of a route with one changing part filled in: `/p/$project` and `WEB` is `/p/WEB`. */
function filled(path: Path, param: RouteParam, value: string): string {
  return path.replace(`$${param}`, encodeURIComponent(value));
}

/** The address of a project: its short name, or an old one, in `/p/WEB`. */
export function projectAddress(shortName: string): string {
  return filled(paths.project, routeParam.project, shortName);
}

/** The address of a page below a project, like `/p/WEB/settings/fields`, with the short name filled in. */
export function addressInProject(path: Extract<Path, `/p/${string}`>, shortName: string): string {
  return filled(path, routeParam.project, shortName);
}

/** The address of the page of a person, `/settings/people/<ID>`. */
export function personAddress(id: string): string {
  return filled(paths.person, routeParam.person, id);
}

/** The address of *Groups* with one group open, `/settings/groups?open=<ID>`. */
export function groupAddress(id: string): string {
  return `${paths.groups}?open=${encodeURIComponent(id)}`;
}

/** The address of the page of a role, `/settings/roles/<ID>`. */
export function roleAddress(id: string): string {
  return filled(paths.role, routeParam.role, id);
}
