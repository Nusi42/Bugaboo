import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Icon,
  iconName,
  iconSize,
  partOf,
  part,
  ShowMoreOf,
  ariaCurrent,
} from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";
import { Button as TreeButton, Link } from "react-aria-components";

import { messageOf } from "../../shared/errors.ts";
import { isAtOrUnder, projectAddress } from "../../shared/paths.ts";
import { NavLink } from "../shell/index.ts";
import { toggleOpened, useOpened } from "./tree-state.ts";
import { linesOf, useProjectList, type ProjectLine } from "./use-projects.ts";

/** Whether the project is the one that is open, or one of its pages. */
function isOpen(project: ProjectLine, pathname: string): boolean {
  return isAtOrUnder(projectAddress(project.shortName), pathname);
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
    <li {...partOf(part.column)}>
      <div {...partOf(part.projectTreeRow)}>
        {project.hasSubProjects ? (
          <TreeButton
            {...partOf(part.projectTreeToggle, { states: [opened && "open"] })}
            aria-expanded={opened}
            aria-label={t("projects.toggle-name", { name: project.name })}
            onPress={() => {
              toggleOpened(project.id);
            }}
          >
            <Icon name={iconName.next} size={iconSize.small} />
          </TreeButton>
        ) : (
          <span {...partOf(part.projectTreeSpacer)} aria-hidden />
        )}
        <Link
          {...partOf(part.projectTreeLink)}
          href={projectAddress(project.shortName)}
          {...(current ? { "aria-current": ariaCurrent.page } : {})}
          {...(onNavigate === undefined ? {} : { onPress: onNavigate })}
        >
          <span {...partOf(part.wrap)}>{project.name}</span>
          <span {...partOf(part.projectTreeShort)}>{project.shortName}</span>
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
    return <Alert tone={alertTone.danger}>{messageOf(list.error)}</Alert>;
  }
  if (list.isPending) {
    return <p {...partOf(part.projectTreeNote)}>{t("common.loading")}</p>;
  }
  if (projects.length === 0 && inside === undefined) {
    return <p {...partOf(part.projectTreeNote)}>{t("projects.tree-empty")}</p>;
  }
  return (
    <>
      <ul {...partOf(part.list, { variants: ["tight", inside !== undefined && "inset"] })}>
        {projects.map((project) => (
          <Node key={project.id} project={project} pathname={pathname} onNavigate={onNavigate} />
        ))}
      </ul>
      <ShowMoreOf label={t("projects.tree-more")} list={list} />
    </>
  );
}

/** The sidebar narrowed to icons: the projects at the top, each with its name in a tooltip. */
function Narrow({ pathname, onNavigate }: Omit<BranchProps, "inside">) {
  const list = useProjectList({});
  const projects = linesOf(list.data?.pages);
  return (
    <ul {...partOf(part.list, { variants: ["tight"] })}>
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
