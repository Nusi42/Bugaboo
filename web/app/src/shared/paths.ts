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
  frames: "/frames",
} as const;
