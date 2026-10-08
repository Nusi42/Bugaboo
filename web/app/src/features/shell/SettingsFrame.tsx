import { t } from "@bugaboo/messages";
import { Card, useLayout } from "@bugaboo/ui";
import { Navigate, Outlet, useLocation } from "@tanstack/react-router";
import { useId } from "react";

import { usePermissions } from "../../shared/use-me.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { NavLink } from "./NavItem.tsx";
import { placeAt, topicGroups, type Place, type TopicGroup } from "../../shared/places.ts";
import styles from "./SettingsFrame.module.css";

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

/** The topics grouped under their headings, as links. */
function Topics({ groups, pathname }: TopicsProps) {
  const names = useId();
  return (
    <ul className={styles.groups}>
      {groups.map((group) => (
        <li key={group.heading} className={styles.group}>
          <span id={headingId(names, group)} className={styles.heading}>
            {t(group.heading)}
          </span>
          <ul className={styles.topics} aria-labelledby={headingId(names, group)}>
            {group.topics.map((topic) => (
              <li key={topic.path}>
                <NavLink
                  href={topic.path}
                  icon={topic.icon}
                  label={t(topic.name)}
                  current={pathname === topic.path}
                />
              </li>
            ))}
          </ul>
        </li>
      ))}
    </ul>
  );
}

/** The page of the settings on a phone: the topics. Only this page names the tab; a topic names its own. */
function TopicPage({
  heading,
  groups,
}: {
  readonly heading: string;
  readonly groups: readonly TopicGroup[];
}) {
  usePageTitle(heading);
  return (
    <Card heading={heading} level={1}>
      <nav aria-label={heading}>
        <Topics groups={groups} />
      </nav>
    </Card>
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
  if (pathname === parent) {
    if (!phone && first !== undefined) {
      return <Navigate to={first.path} replace />;
    }
    return <TopicPage heading={heading} groups={groups} />;
  }
  return phone ? (
    <Outlet />
  ) : (
    <div className={styles.frame}>
      <nav aria-label={heading} className={styles.menu}>
        <Topics groups={groups} pathname={pathname} />
      </nav>
      <div className={styles.content}>
        <Outlet />
      </div>
    </div>
  );
}
