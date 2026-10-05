/**
 * The addresses of the pages of the app, written once. Routes, links and redirects all use these,
 * so an address cannot differ between the page and the link to it (ODR).
 */
export const paths = {
  home: "/",
  setup: "/setup",
  signIn: "/sign-in",
  secondStep: "/sign-in/second-step",
  signUp: "/sign-up",
  invitation: "/invitation",
  security: "/account/security",
  sessions: "/account/sessions",
  invitations: "/admin/invitations",
  status: "/admin/status",
} as const;
