import type { Schema } from "@bugaboo/api-client";
import { IssueTypeIcon, Link, StatusIcon, iconSize, type ListColumn } from "@bugaboo/ui";
import type { ReactNode } from "react";

/** What the first cells of every row of an issue are made of: the list, the slim list, the sub-issues. */
export interface CellSource {
  /** The readable ID. */
  readonly key: string;
  readonly title: string;
  /** The group of its status; nothing where the reader may not see the status. */
  readonly group: Schema<"StatusGroup"> | null | undefined;
  /** The words of its status, where the reader may see it: the icon is named by them. */
  readonly statusWord: string | undefined;
  /** Its issue type; none where the list does not ask for it. */
  readonly type: Pick<Schema<"IssueTypeRef">, "icon" | "name"> | undefined;
}

/** What a row adds to its cells. */
export interface CellOptions {
  /** What stands before the title, in muted text: the meaning of a relation. */
  readonly before?: ReactNode;
  /** The issue is the one that is open: its link is the current page. */
  readonly current?: boolean;
}

/**
 * The cells an issue has in every row that shows it (R9, same things, same places): the status icon,
 * the ID, the type, and the title that opens the issue, with what stands before the title (the
 * meaning of a relation). A row takes the ones its columns have and adds its own.
 */
export function issueCells(
  source: CellSource,
  href: string,
  { before, current = false }: CellOptions = {},
): Readonly<Pick<Record<ListColumn, ReactNode>, "status" | "key" | "type" | "title">> {
  const { key, title, group, statusWord, type } = source;
  return {
    status:
      group === undefined || group === null || statusWord === undefined ? undefined : (
        <StatusIcon group={group} size={iconSize.small} label={statusWord} />
      ),
    key,
    type: type === undefined ? undefined : <IssueTypeIcon name={type.icon} label={type.name} />,
    title: (
      <>
        {before}
        <Link href={href} current={current}>
          {title}
        </Link>
      </>
    ),
  };
}
