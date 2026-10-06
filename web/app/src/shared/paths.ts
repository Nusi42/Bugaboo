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
  project: "/p/$project",
  projectSettings: "/p/$project/settings",
  projectRoles: "/p/$project/settings/roles",
  frames: "/frames",
} as const;

/** The address of a project: its short name, or an old one, in `/p/WEB`. */
export function projectAddress(shortName: string): string {
  return paths.project.replace("$project", encodeURIComponent(shortName));
}

/** The address of the page of a person, `/settings/people/<ID>`. */
export function personAddress(id: string): string {
  return paths.person.replace("$person", encodeURIComponent(id));
}

/** The address of *Groups* with one group open, `/settings/groups?open=<ID>`. */
export function groupAddress(id: string): string {
  return `${paths.groups}?open=${encodeURIComponent(id)}`;
}

/** The address of the page of a role, `/settings/roles/<ID>`. */
export function roleAddress(id: string): string {
  return paths.role.replace("$role", encodeURIComponent(id));
}

/** The address of *Who has which role* in a project, `/p/WEB/settings/roles`. */
export function projectRolesAddress(shortName: string): string {
  return paths.projectRoles.replace("$project", encodeURIComponent(shortName));
}

/** The address of the settings of a project, `/p/WEB/settings`. */
export function projectSettingsAddress(shortName: string): string {
  return paths.projectSettings.replace("$project", encodeURIComponent(shortName));
}
