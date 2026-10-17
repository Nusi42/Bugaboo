import { createRoute, lazyRouteComponent, useNavigate, useSearch } from "@tanstack/react-router";

import { HomePage } from "../features/home/index.ts";
import { paths } from "../shared/paths.ts";
import { shortcutIdOf, type ShortcutId } from "../shared/shortcuts.ts";
import { appRoute, mySettingsRoute, organizationSettingsRoute } from "./layouts.tsx";
import { OnlyWith } from "./OnlyWith.tsx";
import { loadsScreen } from "./preload.ts";
import { ShortcutsRoute, StartPageRoute } from "./settings-routes.tsx";
import { StartScreen } from "./start-screen.ts";

// The pages that are not the first one are loaded when they are first opened, so the first page
// stays small (coding-standards section 6).

const AppearanceScreen = lazyRouteComponent(
  () => import("../features/my-settings/index.ts"),
  "AppearancePage",
);
const RegionScreen = lazyRouteComponent(
  () => import("../features/my-settings/index.ts"),
  "RegionPage",
);
const SecurityScreen = lazyRouteComponent(
  () => import("../features/settings/index.ts"),
  "SecurityPage",
);
const SignInRulesScreen = lazyRouteComponent(
  () => import("../features/settings/index.ts"),
  "SignInRulesPage",
);
const ThemesScreen = lazyRouteComponent(
  () => import("../features/settings/index.ts"),
  "ThemesPage",
);

export const homeRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.home,
  component: HomePage,
});

/** Where *My settings* starts: the frame shows the menu, or goes to the first topic. */
export const mySettingsIndexRoute = createRoute({
  getParentRoute: () => mySettingsRoute,
  path: paths.mySettings,
});

export const appearanceRoute = createRoute({
  getParentRoute: () => mySettingsRoute,
  path: paths.appearance,
  component: AppearanceScreen,
});

export const regionRoute = createRoute({
  getParentRoute: () => mySettingsRoute,
  path: paths.region,
  component: RegionScreen,
});

export const startPageRoute = createRoute({
  getParentRoute: () => mySettingsRoute,
  path: paths.startPage,
  loader: loadsScreen(StartScreen),
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
  loader: loadsScreen(ProjectsScreen),
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
  loader: loadsScreen(StatusScreen),
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
  loader: loadsScreen(SecurityScreen),
  component: function OrganizationSecurity() {
    return (
      <OnlyWith path={paths.organizationSecurity}>
        <SecurityScreen />
      </OnlyWith>
    );
  },
});

export const signInRulesRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.signInRules,
  loader: loadsScreen(SignInRulesScreen),
  component: function SignInRules() {
    return (
      <OnlyWith path={paths.signInRules}>
        <SignInRulesScreen />
      </OnlyWith>
    );
  },
});

export const themesRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.themes,
  loader: loadsScreen(ThemesScreen),
  component: function Themes() {
    return (
      <OnlyWith path={paths.themes}>
        <ThemesScreen />
      </OnlyWith>
    );
  },
});

const NotificationsScreen = lazyRouteComponent(
  () => import("../features/notifications/index.ts"),
  "NotificationsPage",
);
const CustomizeScreen = lazyRouteComponent(
  () => import("../features/notifications/index.ts"),
  "CustomizePage",
);
const MailServerScreen = lazyRouteComponent(
  () => import("../features/notifications/index.ts"),
  "MailServerPage",
);
const HandleScreen = lazyRouteComponent(
  () => import("../features/my-settings/index.ts"),
  "HandlePage",
);

/** *Notifications*: what happened on the issues the person follows and where they are named. */
export const notificationsRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.notifications,
  loader: loadsScreen(NotificationsScreen),
  component: NotificationsScreen,
});

/** *My settings › Notification settings*: what the person hears of, in the app and by e-mail. */
export const notificationSettingsRoute = createRoute({
  getParentRoute: () => mySettingsRoute,
  path: paths.notificationSettings,
  loader: loadsScreen(CustomizeScreen),
  component: CustomizeScreen,
});

/** *My settings › Handle*: the short name people type after the @ to mention the person. */
export const handleRoute = createRoute({
  getParentRoute: () => mySettingsRoute,
  path: paths.handle,
  loader: loadsScreen(HandleScreen),
  component: HandleScreen,
});

/** *Organization settings › Connections › E-mail & notifications*: the mail server. */
export const mailServerRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.mailServer,
  loader: loadsScreen(MailServerScreen),
  component: function MailServer() {
    return (
      <OnlyWith path={paths.mailServer}>
        <MailServerScreen />
      </OnlyWith>
    );
  },
});
