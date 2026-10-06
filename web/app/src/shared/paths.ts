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
  invitations: "/settings/invitations",
  signInRules: "/settings/sign-in-rules",
  themes: "/settings/themes",
  status: "/settings/status",
  projects: "/settings/projects",
  fieldPermissions: "/settings/field-permissions",
  peopleVisibility: "/settings/people-visibility",
  checkAccess: "/settings/check-access",
  project: "/p/$project",
  projectSettings: "/p/$project/settings",
  projectFields: "/p/$project/settings/fields",
  projectPeople: "/p/$project/settings/people",
  projectCheckAccess: "/p/$project/settings/check-access",
  frames: "/frames",
} as const;

/** The address of a project: its short name, or an old one, in `/p/WEB`. */
export function projectAddress(shortName: string): string {
  return paths.project.replace("$project", encodeURIComponent(shortName));
}

/** The address of a page below the settings of a project, `/p/WEB/settings/fields`, with the short name filled in. */
export function projectPageAddress(
  path: typeof paths.projectFields | typeof paths.projectPeople | typeof paths.projectCheckAccess,
  shortName: string,
): string {
  return path.replace("$project", encodeURIComponent(shortName));
}

/** The address of the settings of a project, `/p/WEB/settings`. */
export function projectSettingsAddress(shortName: string): string {
  return paths.projectSettings.replace("$project", encodeURIComponent(shortName));
}
