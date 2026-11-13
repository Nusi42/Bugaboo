import type { Schema } from "@bugaboo/api-client";

import { known } from "../../shared/present.ts";
import { groupOfKey } from "../issues/index.ts";
import type { Span } from "./scale.ts";
import { placeOf } from "./scale.ts";
import type { TimelineGoal, TimelineRow } from "./timeline-data.ts";

/**
 * The lines the chart draws, top to bottom (UX §4.7): the head of each group, foldable, and under
 * it the rows of its issues, in the order the server sent them. Nothing here works out a date: the
 * days of the bars and the goals are the server's (ADR-0030, amendment M8).
 */

/** A group key as the server sends it. */
export type GroupKey = Schema<"SearchGroupKeyBody">;

/** The head of a group: what it is, how many rows it holds, and its goal when it is grouped by goal. */
export interface GroupLine {
  readonly kind: "group";
  readonly id: string;
  readonly key: GroupKey;
  readonly count: number;
  readonly open: boolean;
  readonly goal: TimelineGoal | undefined;
}

/** The row of an issue. */
export interface IssueLine {
  readonly kind: "issue";
  /** Tells the row from every other: an issue has a row in each of its groups. */
  readonly id: string;
  readonly issue: string;
  readonly row: TimelineRow;
}

export type Line = GroupLine | IssueLine;

/** The name of a group the way the list tells groups apart. */
export function groupIdOf(key: GroupKey): string {
  return groupOfKey({ key, filter: null, name: null, issues: 0 }).id;
}

/**
 * The lines of the rows: grouped where the rows have groups, each group where its first row is, a
 * folded group with its head only. Rows without a group stand alone, without a head.
 */
export function linesOf(
  rows: readonly TimelineRow[],
  folded: readonly string[],
  goalOf: (id: string) => TimelineGoal | undefined,
): readonly Line[] {
  const groups = new Map<string, { key: GroupKey | null; rows: TimelineRow[] }>();
  for (const row of rows) {
    const key = row.group;
    const id = key === null ? "" : groupIdOf(key);
    const group = groups.get(id) ?? { key, rows: [] };
    group.rows.push(row);
    groups.set(id, group);
  }
  const lines: Line[] = [];
  for (const [id, group] of groups) {
    const open = !folded.includes(id);
    if (group.key !== null) {
      const { key } = group;
      lines.push({
        kind: "group",
        id,
        key,
        count: group.rows.length,
        open,
        goal: key.type === "issue" ? goalOf(key.value) : undefined,
      });
    }
    if (open) {
      for (const row of group.rows) {
        const issue = known(row.issue.id) ?? "";
        lines.push({ kind: "issue", id: `${id}/${issue}`, issue, row });
      }
    }
  }
  return lines;
}

/** The days of a bar: the server's, or what a drag or a save made of them until the server answers. */
export interface BarDays {
  readonly first: string;
  readonly last: string;
}

/** One arrow: from the end of the bar that must finish first to the start of the bar that waits. */
export interface Arrow {
  readonly key: string;
  /** The line and the day after the last day of the first bar. */
  readonly from: { readonly line: number; readonly day: number };
  /** The line and the first day of the bar that waits. */
  readonly to: { readonly line: number; readonly day: number };
}

/**
 * The arrows between bars that are on screen: an issue in several rows (*Group by person*) is
 * joined from its first row that is not a copy. An arrow to a folded row is not drawn.
 */
export function arrowsOf(
  lines: readonly Line[],
  dependencies: readonly Schema<"TimelineDependencyBody">[],
  daysOf: (line: IssueLine) => BarDays | undefined,
  span: Span,
): readonly Arrow[] {
  const at = new Map<string, number>();
  lines.forEach((line, index) => {
    if (line.kind === "issue" && !at.has(line.issue) && !line.row.copy) {
      at.set(line.issue, index);
    }
  });
  const arrows: Arrow[] = [];
  for (const { first, then } of dependencies) {
    const [fromLine, toLine] = [at.get(first), at.get(then)];
    if (fromLine === undefined || toLine === undefined) {
      continue;
    }
    const [from, to] = [lines[fromLine], lines[toLine]];
    const [fromDays, toDays] = [
      from?.kind === "issue" ? daysOf(from) : undefined,
      to?.kind === "issue" ? daysOf(to) : undefined,
    ];
    if (fromDays === undefined || toDays === undefined) {
      continue;
    }
    arrows.push({
      key: `${first}>${then}`,
      from: { line: fromLine, day: placeOf(span, fromDays.last) + 1 },
      to: { line: toLine, day: placeOf(span, toDays.first) },
    });
  }
  return arrows;
}
