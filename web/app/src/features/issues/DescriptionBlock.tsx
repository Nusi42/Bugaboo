import { descriptionEditPart, type SafeHtml } from "@bugaboo/api-client";
import { partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { CompareAndSave } from "./CompareAndSave.tsx";
import { DescriptionEditor, DescriptionReading } from "./DescriptionEditor.tsx";
import { DescriptionHead } from "./DescriptionHead.tsx";
import { DescriptionEditNotices } from "./DescriptionEditNotices.tsx";
import { descriptionWords, sectionId, type LongTextWords } from "./long-text-words.ts";
import type { IssueView } from "./slots.ts";
import { useDescription, type Description } from "./use-description.ts";

export interface LongTextBlockProps {
  /** The text as a long text being edited. */
  readonly description: Description;
  /** The ID of the issue and the part of it that the text is. */
  readonly issue: string;
  readonly which: string;
  /** The text rendered by the server; nothing when the person may not see it. */
  readonly html: SafeHtml | undefined;
  /** Whether there is a text at all. */
  readonly filled: boolean;
  /** Whether the person may change it. */
  readonly canChange: boolean;
  readonly words: LongTextWords;
}

/**
 * A long text of an issue (ISS-17, ADR-0019): shown as the server rendered it; *Edit* opens the
 * box, the first key takes the lock, and what the person writes is kept as a draft in the browser
 * and on the server until it is saved. When something goes wrong the text stays and the page says
 * what happened and what to do (*Compare and save*, *Ask ... to finish*, *Edit anyway*).
 */
export function LongTextBlock({
  description,
  issue,
  which,
  html,
  filled,
  canChange,
  words,
}: LongTextBlockProps) {
  const [comparing, setComparing] = useState(false);
  const showCompare = comparing && description.phase.kind !== "reading";
  return (
    <section {...partOf(part.issueSection)} aria-labelledby={sectionId(issue, which)}>
      <DescriptionHead
        description={description}
        titleId={sectionId(issue, which)}
        canChange={canChange}
        words={words}
      />
      <DescriptionEditNotices
        description={description}
        issue={issue}
        which={which}
        comparing={showCompare}
        onCompare={() => {
          setComparing(true);
          description.reread();
        }}
      />
      {showCompare ? (
        <CompareAndSave
          mine={description.text}
          theirs={description.saved}
          onSave={(text) => {
            setComparing(false);
            description.resolve(text);
          }}
          onClose={() => {
            setComparing(false);
          }}
        />
      ) : null}
      {description.phase.kind === "reading" ? (
        <DescriptionReading html={html} filled={filled} empty={words.empty} />
      ) : (
        <DescriptionEditor description={description} words={words} />
      )}
    </section>
  );
}

export interface DescriptionBlockProps {
  readonly issue: IssueView;
  /** The address the page was opened with, to read the issue again after a save. */
  readonly address: string;
  /** The description rendered by the server; nothing when the person may not see it. */
  readonly html: SafeHtml | undefined;
  /** Whether the person may change the description. */
  readonly canChange: boolean;
}

/** The description of an issue: a long text with the words of the description. */
export function DescriptionBlock({ issue, address, html, canChange }: DescriptionBlockProps) {
  const description = useDescription(issue, address);
  const filled = (issue.description ?? "") !== "";
  return (
    <LongTextBlock
      description={description}
      issue={issue.id ?? address}
      which={descriptionEditPart}
      html={html}
      filled={filled}
      canChange={canChange}
      words={descriptionWords()}
    />
  );
}
