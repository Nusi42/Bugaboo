import { t, type MessageKey } from "@bugaboo/messages";
import { Card, useLayout, partOf, part, type IconName } from "@bugaboo/ui";
import { Navigate, Outlet, useLocation } from "@tanstack/react-router";
import { useId, type ReactNode } from "react";

import { deepestAround } from "../../shared/paths.ts";
import { usePermissions } from "../../shared/use-me.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { NavLink } from "./NavItem.tsx";
import { placeAt, topicGroups, type Place, type TopicGroup } from "../../shared/places.ts";

export interface SettingsFrameProps {
  /** The settings screen: `/me` or `/settings`. */
  readonly parent: Place["path"];
}

interface TopicsProps {
  readonly groups: readonly TopicGroup[];
  /** The page that is open, whose topic is marked; none on the page that lists them. */
  readonly pathname?: string;
}

/** The id of the heading of a group, which labels the list of its topics. */
function headingId(prefix: string, group: TopicGroup): string {
  return `${prefix}-${group.heading}`;
}

export interface TopicListProps {
  /** Where each goes (an address), what it is called and its icon. */
  readonly topics: readonly {
    readonly path: string;
    readonly name: MessageKey;
    readonly icon: IconName;
  }[];
  /** The page that is open, whose topic (the deepest that holds it) is marked; none on the screen of the menu. */
  readonly pathname?: string | undefined;
  /** The ID of what names the list, for screen readers. */
  readonly labelledBy?: string;
}

/** A list of topics as links with their icons: the one menu of every settings screen. */
export function TopicList({ topics, pathname, labelledBy }: TopicListProps) {
  // On a phone the menu is a page of its own, and each topic opens one: the chevron says so.
  const phone = useLayout() === "phone";
  const current = deepestAround(
    topics.map((topic) => topic.path),
    pathname,
  );
  return (
    <ul
      {...partOf(part.settingsFrameTopics)}
      {...(labelledBy === undefined ? {} : { "aria-labelledby": labelledBy })}
    >
      {topics.map((topic) => (
        <li key={topic.path}>
          <NavLink
            href={topic.path}
            icon={topic.icon}
            label={t(topic.name)}
            current={current === topic.path}
            opens={phone}
          />
        </li>
      ))}
    </ul>
  );
}

/** The topics grouped under their headings, as links. */
function Topics({ groups, pathname }: TopicsProps) {
  const names = useId();
  // One topic of all the groups is marked: the deepest that holds the open page.
  const open = deepestAround(
    groups.flatMap((group) => group.topics.map((topic) => topic.path)),
    pathname,
  );
  return (
    <ul {...partOf(part.settingsFrameGroups)}>
      {groups.map((group) => (
        <li key={group.heading} {...partOf(part.column)}>
          <span id={headingId(names, group)} {...partOf(part.settingsFrameHeading)}>
            {t(group.heading)}
          </span>
          <TopicList topics={group.topics} pathname={open} labelledBy={headingId(names, group)} />
        </li>
      ))}
    </ul>
  );
}

/** The screen of the menu: only it names the tab; a topic names its own. */
function MenuScreen({
  heading,
  label,
  menu,
}: {
  readonly heading: string;
  readonly label: string;
  readonly menu: ReactNode;
}) {
  usePageTitle(heading);
  return (
    <Card heading={heading} level={1}>
      <nav aria-label={label}>{menu}</nav>
    </Card>
  );
}

export interface SettingsScreensProps {
  /** What the settings are called: the heading of the menu screen and the name of the menu. */
  readonly heading: string;
  /** What names the menu for screen readers, where it is another than the heading. */
  readonly label?: string;
  /** The menu of the topics; none where there is only one topic, and no menu is drawn. */
  readonly menu?: ReactNode;
  /** This is the screen of the menu itself (a phone shows the menu and a topic one at a time). */
  readonly onMenu: boolean;
  /** The open topic. */
  readonly children: ReactNode;
}

/**
 * The one rule of every settings screen: the menu is a screen of its own (`onMenu`), or a topic is
 * shown, with the menu at its side on a computer and with *Back* on a phone, where one screen shows
 * at a time.
 */
export function SettingsScreens({ heading, label, menu, onMenu, children }: SettingsScreensProps) {
  const phone = useLayout() === "phone";
  if (onMenu) {
    return <MenuScreen heading={heading} label={label ?? heading} menu={menu} />;
  }
  if (phone || menu === undefined) {
    return children;
  }
  return (
    <div {...partOf(part.settingsFrame)}>
      <nav aria-label={label ?? heading} {...partOf(part.settingsFrameMenu)}>
        {menu}
      </nav>
      <div {...partOf(part.shellContent)}>{children}</div>
    </div>
  );
}

/**
 * The frame of *My settings* and *Organization settings* (UX §5.1): a menu on the left, by topic
 * and in plain words, and the topic on the right. Only topics that exist are listed. On a phone
 * one screen shows at a time: the menu is the page of the settings itself, and a topic has *Back*.
 */
export function SettingsFrame({ parent }: SettingsFrameProps) {
  const { pathname } = useLocation();
  const phone = useLayout() === "phone";
  const permissions = usePermissions();
  const groups = topicGroups(parent, permissions);
  const here = placeAt(parent);
  const heading = here === undefined ? t("app.name") : t(here.name);
  const first = groups[0]?.topics[0];
  if (pathname === parent && !phone && first !== undefined) {
    return <Navigate to={first.path} replace />;
  }
  return (
    <SettingsScreens
      heading={heading}
      menu={<Topics groups={groups} pathname={pathname} />}
      onMenu={pathname === parent}
    >
      <Outlet />
    </SettingsScreens>
  );
}
