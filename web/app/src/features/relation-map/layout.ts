import type { Schema } from "@bugaboo/api-client";

/**
 * The layout of the relation map (ISS-16, UX §4.15), as ADR-0009, amendment 2026-11-24, item 2
 * decides it: the start on the left, one column for each step away to its right, and in each column
 * the issues in the order of the answer, which is the fixed order of the server (by kind of
 * relation, then by Issue ID). Nothing here is random and nothing settles, so the same answer always
 * gives the same picture (UX §6.4). The map of a search has no start: each of its groups of
 * connected results starts at its first result, in the order of the search.
 */

/** The relation map as the person may see it. */
export type MapAnswer = Schema<"RelationMapBody">;

/** A line between two issues on the map. */
export type MapLine = Schema<"MapLineBody">;

/** An issue on the map, with what the person may see of it. */
export type MapIssue = Schema<"IssueRef">;

/** An issue where the picture puts it: its column (steps away) and its row in that column. */
export interface MapCard {
  readonly issue: MapIssue;
  readonly column: number;
  readonly row: number;
}

/** One relation along a chain, in the words of the issue it starts from: *WEB-142 blocks WEB-147*. */
export interface ChainStep {
  readonly from: MapIssue;
  readonly word: string;
  readonly to: MapIssue;
  readonly line: MapLine;
}

/** The picture of an answer: where every issue stands, and how each one is reached. */
export interface MapPicture {
  /** The internal ID of the start; none on the map of a search. */
  readonly start: string | undefined;
  /** The cards of each column, from the left, each column in its fixed order. */
  readonly columns: readonly (readonly MapCard[])[];
  /** Every card by the internal ID of its issue. */
  readonly cards: Readonly<Record<string, MapCard>>;
  /** The lines of the answer, in its order. */
  readonly lines: readonly MapLine[];
  /** For every issue right of the first column, the line that reaches it from the column before. */
  readonly reached: Readonly<Record<string, MapLine>>;
}

/** The key of a line: the two issues and the kind; *part of* has no kind. */
export function lineKey(line: MapLine): string {
  return `${line.from}>${line.to}>${line.relation_kind ?? ""}`;
}

/** The issue at the other end of a line from `id`. */
function otherEnd(line: MapLine, id: string): string | undefined {
  if (line.from === id) {
    return line.to;
  }
  return line.to === id ? line.from : undefined;
}

/** What `id` reads at its end of the line: the forward words when it is the one the line starts from. */
export function wordOf(line: MapLine, id: string): string {
  return line.from === id ? line.forward : line.backward;
}

/** The lines at an issue, in the order of the answer. */
type LinesAt = (id: string) => readonly MapLine[];

/** The lines at each issue, in the order of the answer. */
function linesAt(lines: readonly MapLine[]): LinesAt {
  const at = new Map<string, MapLine[]>();
  for (const line of lines) {
    for (const end of [line.from, line.to]) {
      const list = at.get(end) ?? [];
      list.push(line);
      at.set(end, list);
    }
  }
  return (id) => at.get(id) ?? [];
}

/** How many steps away an issue stands, by its internal ID. */
type StepsOf = (id: string) => number | undefined;

/** The column of each issue of the map of an issue: its steps away, which the server counted. */
function stepsOfIssueMap(answer: MapAnswer): StepsOf {
  const steps = new Map(answer.issues.map((each) => [each.issue.id, each.steps ?? 0]));
  return (id) => steps.get(id);
}

/**
 * The column of each result of the map of a search: each group of connected results starts at its
 * first result in the order of the search, and the others stand by how many lines lie between.
 */
function stepsOfSearchMap(answer: MapAnswer, at: LinesAt): StepsOf {
  const known = new Set(answer.issues.map((each) => each.issue.id));
  const steps = new Map<string, number>();
  for (const { issue } of answer.issues) {
    if (steps.has(issue.id)) {
      continue;
    }
    steps.set(issue.id, 0);
    const queue = [issue.id];
    for (let next = queue.shift(); next !== undefined; next = queue.shift()) {
      const here = steps.get(next) ?? 0;
      for (const line of at(next)) {
        const other = otherEnd(line, next);
        if (other !== undefined && known.has(other) && !steps.has(other)) {
          steps.set(other, here + 1);
          queue.push(other);
        }
      }
    }
  }
  return (id) => steps.get(id);
}

/** For every issue right of the first column, the first line of the answer that reaches it from the column before. */
function reachedOf(answer: MapAnswer, steps: StepsOf, at: LinesAt): Record<string, MapLine> {
  const reached: Record<string, MapLine> = {};
  for (const { issue } of answer.issues) {
    const here = steps(issue.id) ?? 0;
    const line = at(issue.id).find((each) => {
      const other = otherEnd(each, issue.id);
      return other !== undefined && steps(other) === here - 1;
    });
    if (here > 0 && line !== undefined) {
      reached[issue.id] = line;
    }
  }
  return reached;
}

/** Lays the answer out: the same answer always gives the same picture. */
export function pictureOf(answer: MapAnswer): MapPicture {
  const at = linesAt(answer.lines);
  const steps =
    answer.start === undefined || answer.start === null
      ? stepsOfSearchMap(answer, at)
      : stepsOfIssueMap(answer);
  const columns: MapCard[][] = [];
  const cards: Record<string, MapCard> = {};
  for (const { issue } of answer.issues) {
    const column = steps(issue.id) ?? 0;
    while (columns.length <= column) {
      columns.push([]);
    }
    const cardsHere = columns[column] ?? [];
    const card: MapCard = { issue, column, row: cardsHere.length };
    cardsHere.push(card);
    cards[issue.id] = card;
  }
  return {
    start: answer.start ?? undefined,
    columns,
    cards,
    lines: answer.lines.filter(
      (line) => Object.hasOwn(cards, line.from) && Object.hasOwn(cards, line.to),
    ),
    reached: reachedOf(answer, steps, at),
  };
}

/** The chain from the first column to an issue, one relation a step; none for an issue of the first column. */
export function chainOf(picture: MapPicture, id: string): readonly ChainStep[] {
  const steps: ChainStep[] = [];
  const seen = new Set<string>();
  let here = id;
  for (let line = picture.reached[here]; line !== undefined; line = picture.reached[here]) {
    const before = otherEnd(line, here);
    const from = before === undefined ? undefined : picture.cards[before]?.issue;
    const to = picture.cards[here]?.issue;
    if (before === undefined || from === undefined || to === undefined || seen.has(before)) {
      break;
    }
    seen.add(here);
    steps.unshift({ from, word: wordOf(line, before), to, line });
    here = before;
  }
  return steps;
}

/** The issues (by internal ID) and the lines (by key) of the chain to an issue, the issue itself included. */
export interface OnChain {
  readonly issues: readonly string[];
  readonly lines: readonly string[];
}

/** What is on the chain to an issue. */
export function onChainOf(picture: MapPicture, id: string): OnChain {
  const chain = chainOf(picture, id);
  return {
    issues: [id, ...chain.map((step) => step.from.id)],
    lines: chain.map((step) => lineKey(step.line)),
  };
}

/**
 * A loop of dependencies the server found, as relations: each issue to the next, and the last back
 * to the first. None when a relation of it is not among the lines (its kind is not shown).
 */
export function loopSteps(
  picture: MapPicture,
  loop: readonly string[],
): readonly ChainStep[] | undefined {
  const steps: ChainStep[] = [];
  for (const [at, id] of loop.entries()) {
    const next = loop[(at + 1) % loop.length];
    const from = picture.cards[id]?.issue;
    const to = next === undefined ? undefined : picture.cards[next]?.issue;
    const line =
      picture.lines.find((each) => each.from === id && each.to === next) ??
      picture.lines.find((each) => each.from === next && each.to === id);
    if (from === undefined || to === undefined || line === undefined) {
      return undefined;
    }
    steps.push({ from, word: wordOf(line, id), to, line });
  }
  return steps;
}
