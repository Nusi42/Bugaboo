import { iconName } from "@bugaboo/ui/icons";

import { paths } from "./paths.ts";
import type { Place } from "./place.ts";

/** The screens of the person themselves: the start page and *My settings*. */
export const ownPlaces: readonly Place[] = [
  {
    path: paths.home,
    name: "nav.home",
    icon: iconName.home,
    parent: null,
    listed: true,
  },
  {
    path: paths.notifications,
    name: "nav.notifications",
    icon: iconName.bell,
    // A destination of the bottom bar of a phone, like the start page: it has no Back.
    parent: null,
    listed: true,
  },
  {
    path: paths.time,
    name: "nav.timesheet",
    icon: iconName.clock,
    // A destination of the menu like the start page: it has no Back.
    parent: null,
    listed: true,
  },
  {
    // *Working hours of others*: one person's week, only for a holder of the permission.
    path: paths.timeOthers,
    name: "time.others.title",
    icon: iconName.clock,
    parent: paths.time,
    anyOf: ["see-working-hours-of-others"],
    listed: true,
  },
  {
    path: paths.timeOfPerson,
    name: "time.others.title",
    icon: iconName.clock,
    parent: paths.timeOthers,
    anyOf: ["see-working-hours-of-others"],
    listed: false,
  },
  {
    // *My agents*: in the sidebar and the sheet *More* only for a person who has an agent or may make one.
    path: paths.myAgents,
    name: "nav.my-agents",
    icon: iconName.agent,
    parent: null,
    listed: false,
  },
  {
    path: paths.agent,
    name: "agents.page",
    icon: iconName.agent,
    parent: paths.myAgents,
    listed: false,
  },
  {
    path: paths.mySettings,
    name: "nav.my-settings",
    icon: iconName.settings,
    parent: paths.home,
    listed: true,
  },
  {
    path: paths.appearance,
    name: "nav.appearance",
    icon: iconName.palette,
    parent: paths.mySettings,
    group: "nav.group.look",
    listed: true,
  },
  {
    path: paths.region,
    name: "nav.region",
    icon: iconName.region,
    parent: paths.mySettings,
    group: "nav.group.look",
    listed: true,
  },
  {
    path: paths.startPage,
    name: "nav.start-page",
    icon: iconName.home,
    parent: paths.mySettings,
    group: "nav.group.look",
    listed: true,
  },
  {
    path: paths.shortcuts,
    name: "shortcut.title",
    icon: iconName.keyboard,
    parent: paths.mySettings,
    group: "nav.group.look",
    listed: true,
  },
  {
    path: paths.notificationSettings,
    name: "nav.notification-settings",
    icon: iconName.bell,
    parent: paths.mySettings,
    group: "nav.group.hearing",
    listed: true,
  },
  {
    path: paths.handle,
    name: "nav.handle",
    icon: iconName.people,
    parent: paths.mySettings,
    group: "nav.group.hearing",
    listed: true,
  },
  {
    path: paths.security,
    name: "nav.security",
    icon: iconName.security,
    parent: paths.mySettings,
    group: "nav.group.sign-in",
    listed: true,
  },
  {
    path: paths.sessions,
    name: "nav.sessions",
    icon: iconName.sessions,
    parent: paths.mySettings,
    group: "nav.group.sign-in",
    listed: true,
  },
  {
    path: paths.accessKeys,
    name: "nav.access-keys",
    icon: iconName["access-keys"],
    parent: paths.mySettings,
    group: "nav.group.sign-in",
    listed: true,
  },
  {
    path: paths.connectedApps,
    name: "nav.connected-apps",
    icon: iconName["connected-apps"],
    parent: paths.mySettings,
    group: "nav.group.sign-in",
    listed: true,
  },
  {
    path: paths.assistantSettings,
    name: "nav.assistant-settings",
    icon: iconName.assistant,
    parent: paths.mySettings,
    group: "nav.group.hearing",
    listed: true,
  },
];
