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
  shortcuts: "/me/shortcuts",
  security: "/me/security",
  sessions: "/me/sessions",
  accessKeys: "/me/access-keys",
  organizationSettings: "/settings",
  invitations: "/settings/invitations",
  status: "/settings/status",
  projects: "/settings/projects",
  project: "/p/$project",
  projectSettings: "/p/$project/settings",
  frames: "/frames",
} as const;

/** The address of a project: its short name, or an old one, in `/p/WEB`. */
export function projectAddress(shortName: string): string {
  return paths.project.replace("$project", encodeURIComponent(shortName));
}

/** The address of the settings of a project, `/p/WEB/settings`. */
export function projectSettingsAddress(shortName: string): string {
  return paths.projectSettings.replace("$project", encodeURIComponent(shortName));
}
