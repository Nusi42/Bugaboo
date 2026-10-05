import { t } from "@bugaboo/messages";
import { Alert, Button, cx, Icon, iconName } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";
import { Button as TreeButton, Link } from "react-aria-components";

import { messageOf } from "../../shared/errors.ts";
import { projectAddress } from "../../shared/paths.ts";
import { NavLink } from "../shell/index.ts";
import styles from "./ProjectTree.module.css";
import { toggleOpened, useOpened } from "./tree-state.ts";
import { linesOf, useProjectList, type ProjectLine } from "./use-projects.ts";

/** Words of the ARIA standard, not text for people. */
const currentPage = "page";

/** Whether the project is the one that is open, or one of its pages. */
function isOpen(project: ProjectLine, pathname: string): boolean {
  const address = projectAddress(project.shortName);
  return pathname === address || pathname.startsWith(`${address}/`);
}

interface NodeProps {
  readonly project: ProjectLine;
  readonly pathname: string;
  readonly onNavigate?: (() => void) | undefined;
}

/** One project of the tree: its link, a button that opens what is inside it, and what is inside. */
function Node({ project, pathname, onNavigate }: NodeProps) {
  const opened = useOpened(project.id);
  const current = isOpen(project, pathname);
  return (
    <li className={styles.node}>
      <div className={styles.row}>
        {project.hasSubProjects ? (
          <TreeButton
            className={cx(styles.toggle)}
            aria-expanded={opened}
            aria-label={t("projects.toggle-name", { name: project.name })}
            onPress={() => {
              toggleOpened(project.id);
            }}
          >
            <Icon name={iconName.next} small />
          </TreeButton>
        ) : (
          <span className={styles.spacer} aria-hidden />
        )}
        <Link
          className={cx(styles.link)}
          href={projectAddress(project.shortName)}
          {...(current ? { "aria-current": currentPage } : {})}
          {...(onNavigate === undefined ? {} : { onPress: onNavigate })}
        >
          <span className={styles.name}>{project.name}</span>
          <span className={styles.short}>{project.shortName}</span>
        </Link>
      </div>
      {opened ? <Branch inside={project.id} pathname={pathname} onNavigate={onNavigate} /> : null}
    </li>
  );
}

interface BranchProps {
  /** The project whose sub-projects are listed; the top of the tree without it. */
  readonly inside?: string;
  readonly pathname: string;
  readonly onNavigate?: (() => void) | undefined;
}

/** The projects directly inside one project (or at the top), a page at a time. */
function Branch({ inside, pathname, onNavigate }: BranchProps) {
  const list = useProjectList(inside === undefined ? {} : { inside });
  const projects = linesOf(list.data?.pages);
  if (list.isError) {
    return <Alert danger>{messageOf(list.error)}</Alert>;
  }
  if (list.isPending) {
    return <p className={styles.note}>{t("common.loading")}</p>;
  }
  if (projects.length === 0 && inside === undefined) {
    return <p className={styles.note}>{t("projects.tree-empty")}</p>;
  }
  return (
    <>
      <ul className={inside === undefined ? styles.top : styles.inside}>
        {projects.map((project) => (
          <Node key={project.id} project={project} pathname={pathname} onNavigate={onNavigate} />
        ))}
      </ul>
      {list.hasNextPage ? (
        <Button
          quiet
          isPending={list.isFetchingNextPage}
          onPress={() => {
            void list.fetchNextPage();
          }}
        >
          {t("projects.tree-more")}
        </Button>
      ) : null}
    </>
  );
}

/** The sidebar narrowed to icons: the projects at the top, each with its name in a tooltip. */
function Narrow({ pathname, onNavigate }: Omit<BranchProps, "inside">) {
  const list = useProjectList({});
  const projects = linesOf(list.data?.pages);
  return (
    <ul className={styles.narrow}>
      {projects.map((project) => (
        <li key={project.id}>
          <NavLink
            href={projectAddress(project.shortName)}
            icon={iconName.projects}
            label={project.name}
            narrow
            current={isOpen(project, pathname)}
            {...(onNavigate === undefined ? {} : { onNavigate })}
          />
        </li>
      ))}
    </ul>
  );
}

/**
 * The project tree of the sidebar (UX §3.1, PRJ-02): the projects the person sees, each opening to
 * the projects inside it, one level and one page at a time. What is open is remembered in this
 * browser. A visible project whose parent is hidden is at the top; nothing counts what is inside.
 */
export function ProjectTree({
  narrow,
  onNavigate,
}: {
  readonly narrow: boolean;
  readonly onNavigate?: () => void;
}) {
  const { pathname } = useLocation();
  return narrow ? (
    <Narrow pathname={pathname} onNavigate={onNavigate} />
  ) : (
    <Branch pathname={pathname} onNavigate={onNavigate} />
  );
}
