import type { SafeHtml } from "@bugaboo/api-client";

import { LongTextBlock } from "./DescriptionBlock.tsx";
import { sectionId, type LongTextWords } from "./long-text-words.ts";
import type { IssueView } from "./slots.ts";
import { useIssueLongText, type SaveLongText } from "./use-description.ts";

export interface IssueLongTextProps {
  readonly issue: IssueView;
  /** The address the page was opened with, to read the issue again after a save. */
  readonly address: string;
  /** The name of the part the lock and the drafts are about. */
  readonly part: string;
  /** The text as saved now (Markdown). */
  readonly text: string;
  /** The text rendered by the server; nothing when there is none. */
  readonly html: SafeHtml | undefined;
  readonly canChange: boolean;
  readonly words: Omit<LongTextWords, "shortcut">;
  /** Saves the text under the lock; stays the same while the text is on screen. */
  readonly save: SaveLongText;
}

/**
 * A long text of an issue that another package owns (a long-text own field): the same box, lock,
 * drafts and notes as the description, with its own words and its own save.
 */
export function IssueLongText({
  issue,
  address,
  part,
  text,
  html,
  canChange,
  words,
  save,
}: IssueLongTextProps) {
  const long = useIssueLongText(issue, address, { part, saved: text, save });
  const filled = text !== "";
  return (
    <LongTextBlock
      description={long}
      titleId={sectionId(issue.id ?? address, part)}
      html={html}
      filled={filled}
      canChange={canChange}
      words={{ ...words, shortcut: false }}
    />
  );
}
