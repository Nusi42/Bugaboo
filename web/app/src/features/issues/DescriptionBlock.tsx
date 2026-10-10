import type { SafeHtml } from "@bugaboo/api-client";
import { partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { CompareAndSave } from "./CompareAndSave.tsx";
import { DescriptionEditor, DescriptionReading } from "./DescriptionEditor.tsx";
import { DescriptionHead } from "./DescriptionHead.tsx";
import { DescriptionNotes } from "./DescriptionNotes.tsx";
import type { IssueView } from "./slots.ts";
import { useDescription } from "./use-description.ts";

export interface DescriptionBlockProps {
  readonly issue: IssueView;
  /** The address the page was opened with, to read the issue again after a save. */
  readonly address: string;
  /** The description rendered by the server; nothing when the person may not see it. */
  readonly html: SafeHtml | undefined;
  /** Whether the person may change the description. */
  readonly canChange: boolean;
}

/**
 * The description of an issue (ISS-17, ADR-0019): shown as the server rendered it; *Edit* opens the
 * box, the first key takes the lock, and what the person writes is kept as a draft in the browser
 * and on the server until it is saved. When something goes wrong the text stays and the page says
 * what happened and what to do (*Compare and save*, *Ask ... to finish*, *Edit anyway*).
 */
export function DescriptionBlock({ issue, address, html, canChange }: DescriptionBlockProps) {
  const description = useDescription(issue, address);
  const [comparing, setComparing] = useState(false);
  const editing = description.phase.kind !== "reading";
  const showCompare = comparing && editing;
  const titleId = `${issue.id ?? address}-description`;
  const filled = (issue.description ?? "") !== "";
  return (
    <section {...partOf(part.issueSection)} aria-labelledby={titleId}>
      <DescriptionHead description={description} titleId={titleId} canChange={canChange} />
      <DescriptionNotes
        description={description}
        comparing={showCompare}
        onCompare={() => {
          setComparing(true);
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
      {editing ? (
        <DescriptionEditor description={description} />
      ) : (
        <DescriptionReading html={html} filled={filled} />
      )}
    </section>
  );
}
