import { useId, useState, type ReactNode } from "react";

import { Button } from "./Button.tsx";
import { buttonLook } from "./props.ts";
import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

/** One line under the heading of a group of a long preview. */
export interface LongPreviewLine {
  /** Names the line among the others of its group. */
  readonly key: string;
  readonly content: ReactNode;
}

/** The things of one issue or one person in a long preview, with what happens to them. */
export interface LongPreviewGroup {
  /** Names the group among the others. */
  readonly key: string;
  /** Who or what the lines are about: an issue, a person. */
  readonly heading: ReactNode;
  readonly lines: readonly LongPreviewLine[];
  /** The group is left alone (skipped): it reads quieter. */
  readonly skipped?: boolean | undefined;
}

export interface LongPreviewProps {
  /** The sentence on top: "118 will change, 2 will be skipped". */
  readonly summary: string;
  /** The things, grouped; the screen shows the first ones and *Show all* opens the rest. */
  readonly groups: readonly LongPreviewGroup[];
  /** The name of the list for people who read it with a screen reader. */
  readonly label: string;
  /** How many groups are shown before *Show all*; 10 when left out. */
  readonly first?: number | undefined;
  /** *Show all 118*: the button that opens the rest, for the number of groups. */
  readonly showAll: (count: number) => string;
  /** *Show fewer*: the button that closes them again. */
  readonly showFewer: string;
}

/** How many groups a long preview shows before *Show all*. */
const SHOWN_FIRST = 10;

/** One group of a long preview: who or what it is about, and the lines under it. */
function Group({ group }: { readonly group: LongPreviewGroup }) {
  return (
    <li {...partOf(part.longPreviewGroup, { states: [group.skipped === true && "skipped"] })}>
      <div {...partOf(part.longPreviewGroupHead)}>{group.heading}</div>
      {group.lines.length === 0 ? null : (
        <ul {...partOf(part.longPreviewLines)}>
          {group.lines.map((line) => (
            <li key={line.key}>{line.content}</li>
          ))}
        </ul>
      )}
    </li>
  );
}

/**
 * One component for every long preview (M2C-F80, UX principle 6): a summary first (which also says when there is nothing to show), then the things
 * grouped by issue or by person, a few at a time with *Show all*. Moving a project, changing many
 * issues, and renaming or combining a tag or a label all use it, so they read alike.
 */
export function LongPreview({
  summary,
  groups,
  label,
  first = SHOWN_FIRST,
  showAll,
  showFewer,
}: LongPreviewProps) {
  const [open, setOpen] = useState(false);
  const listId = useId();
  const shown = open ? groups : groups.slice(0, first);
  return (
    <section {...partOf(part.longPreview)}>
      <output {...partOf(part.longPreviewSummary)}>{summary}</output>
      {groups.length === 0 ? null : (
        <ul id={listId} {...partOf(part.longPreviewGroups)} aria-label={label}>
          {shown.map((group) => (
            <Group key={group.key} group={group} />
          ))}
        </ul>
      )}
      {groups.length > first ? (
        <div {...partOf(part.row)}>
          <Button
            look={buttonLook.quiet}
            aria-expanded={open}
            aria-controls={listId}
            onPress={() => {
              setOpen(!open);
            }}
          >
            {open ? showFewer : showAll(groups.length)}
          </Button>
        </div>
      ) : null}
    </section>
  );
}
