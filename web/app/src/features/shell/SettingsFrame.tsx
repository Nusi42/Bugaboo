import { t } from "@bugaboo/messages";
import { Card, useLayout } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";
import { Navigate, Outlet, useLocation } from "@tanstack/react-router";

import { meOptions } from "../../shared/session.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { NavLink } from "./NavItem.tsx";
import { placeAt, topicGroups, type Place, type TopicGroup } from "./places.ts";
import styles from "./SettingsFrame.module.css";

export interface SettingsFrameProps {
  /** The settings screen: `/me` or `/settings`. */
  readonly parent: Place["path"];
}

interface TopicsProps {
  readonly groups: readonly TopicGroup[];
  readonly pathname: string;
}

/** The topics grouped under their headings, as links. */
function Topics({ groups, pathname }: TopicsProps) {
  return (
    <ul className={styles.groups}>
      {groups.map((group) => (
        <li key={group.heading} className={styles.group}>
          <span className={styles.heading}>{t(group.heading)}</span>
          <ul className={styles.topics}>
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

/**
 * The frame of *My settings* and *Organization settings* (UX §5.1): a menu on the left, by topic
 * and in plain words, and the topic on the right. Only topics that exist are listed. On a phone
 * one screen shows at a time: the menu is the page of the settings itself, and a topic has *Back*.
 */
export function SettingsFrame({ parent }: SettingsFrameProps) {
  const { pathname } = useLocation();
  const phone = useLayout() === "phone";
  const administrator = useQuery(meOptions()).data?.administrator === true;
  const groups = topicGroups(parent, administrator);
  const here = placeAt(parent);
  const heading = here === undefined ? t("app.name") : t(here.name);
  usePageTitle(pathname === parent ? heading : t("app.name"));
  const first = groups[0]?.topics[0];
  if (pathname === parent) {
    if (!phone && first !== undefined) {
      return <Navigate to={first.path} replace />;
    }
    return (
      <Card heading={heading} level={1}>
        <nav aria-label={heading}>
          <Topics groups={groups} pathname={pathname} />
        </nav>
      </Card>
    );
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
