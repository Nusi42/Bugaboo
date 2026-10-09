import { createRoute, lazyRouteComponent, useNavigate, useSearch } from "@tanstack/react-router";

import { HomePage } from "../features/home/index.ts";
import { AppearancePage, RegionPage } from "../features/my-settings/index.ts";
import { SecurityPage, SignInRulesPage, ThemesPage } from "../features/settings/index.ts";
import { type FramesShow } from "../features/shell/index.ts";
import { paths } from "../shared/paths.ts";
import { shortcutIdOf, type ShortcutId } from "../shared/shortcuts.ts";
import { appRoute, mySettingsRoute, organizationSettingsRoute } from "./layouts.tsx";
import { OnlyWith } from "./OnlyWith.tsx";
import { FramesRoute, ShortcutsRoute, StartPageRoute } from "./settings-routes.tsx";

// The pages that are not the first one are loaded when they are first opened, so the first page
// stays small (coding-standards section 6).

export const homeRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.home,
  component: HomePage,
});

/** The address `?show=panel&item=2`: how an example is shown, and which one. */
function isFramesShow(value: unknown): value is FramesShow {
  return value === "panel" || value === "focus";
}

/** The frames that issues, documents and settings will open in, as empty frames to try. */
export const framesRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.frames,
  validateSearch: (
    search: Readonly<Record<string, unknown>>,
  ): { show?: FramesShow | undefined; item?: string | undefined } => ({
    show: isFramesShow(search["show"]) ? search["show"] : undefined,
    item: typeof search["item"] === "string" ? search["item"] : undefined,
  }),
  component: FramesRoute,
});

/** Where *My settings* starts: the frame shows the menu, or goes to the first topic. */
export const mySettingsIndexRoute = createRoute({
  getParentRoute: () => mySettingsRoute,
  path: paths.mySettings,
});

export const appearanceRoute = createRoute({
  getParentRoute: () => mySettingsRoute,
  path: paths.appearance,
  component: AppearancePage,
});

export const regionRoute = createRoute({
  getParentRoute: () => mySettingsRoute,
  path: paths.region,
  component: RegionPage,
});

export const startPageRoute = createRoute({
  getParentRoute: () => mySettingsRoute,
  path: paths.startPage,
  component: StartPageRoute,
});

export const shortcutsRoute = createRoute({
  getParentRoute: () => mySettingsRoute,
  path: paths.shortcuts,
  validateSearch: (
    search: Readonly<Record<string, unknown>>,
  ): { edit?: ShortcutId | undefined } => ({ edit: shortcutIdOf(search["edit"]) }),
  component: ShortcutsRoute,
});

export const securityRoute = createRoute({
  getParentRoute: () => mySettingsRoute,
  path: paths.security,
  component: lazyRouteComponent(() => import("../features/account/index.ts"), "SecurityPage"),
});

export const sessionsRoute = createRoute({
  getParentRoute: () => mySettingsRoute,
  path: paths.sessions,
  component: lazyRouteComponent(() => import("../features/account/index.ts"), "SessionsPage"),
});

export const accessKeysRoute = createRoute({
  getParentRoute: () => mySettingsRoute,
  path: paths.accessKeys,
  component: lazyRouteComponent(() => import("../features/my-settings/index.ts"), "AccessKeysPage"),
});

/** Where *Organization settings* starts. */
export const organizationSettingsIndexRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.organizationSettings,
});

const StatusScreen = lazyRouteComponent(() => import("../features/admin/index.ts"), "StatusPage");
const ProjectsScreen = lazyRouteComponent(
  () => import("../features/projects/index.ts"),
  "ProjectsPage",
);

/** *Organization settings › Projects*: the address says which project is open in the side panel. */
export const projectsRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.projects,
  validateSearch: (search: Readonly<Record<string, unknown>>): { open?: string | undefined } => ({
    open: typeof search["open"] === "string" ? search["open"] : undefined,
  }),
  component: function Projects() {
    const { open } = useSearch({ strict: false });
    const navigate = useNavigate();
    return (
      <OnlyWith path={paths.projects}>
        <ProjectsScreen
          open={open}
          onOpen={(shortName: string) => {
            void navigate({ to: paths.projects, search: { open: shortName } });
          }}
          onClose={() => {
            void navigate({ to: paths.projects, search: {} });
          }}
        />
      </OnlyWith>
    );
  },
});

export const statusRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.status,
  component: function Status() {
    return (
      <OnlyWith path={paths.status}>
        <StatusScreen />
      </OnlyWith>
    );
  },
});

export const organizationSecurityRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.organizationSecurity,
  component: function OrganizationSecurity() {
    return (
      <OnlyWith path={paths.organizationSecurity}>
        <SecurityPage />
      </OnlyWith>
    );
  },
});

export const signInRulesRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.signInRules,
  component: function SignInRules() {
    return (
      <OnlyWith path={paths.signInRules}>
        <SignInRulesPage />
      </OnlyWith>
    );
  },
});

export const themesRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.themes,
  component: function Themes() {
    return (
      <OnlyWith path={paths.themes}>
        <ThemesPage />
      </OnlyWith>
    );
  },
});
