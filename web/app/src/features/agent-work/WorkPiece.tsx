import type { Schema } from "@bugaboo/api-client";
import { partOf, part } from "@bugaboo/ui";

import { AttemptLines } from "./AttemptLines.tsx";
import type { Work } from "./agent-work.ts";
import { Questions } from "./QuestionAnswer.tsx";
import { Reviews } from "./Reviews.tsx";
import { WorkActions } from "./WorkActions.tsx";
import { WorkBrief } from "./WorkBrief.tsx";
import { WorkFoot } from "./WorkFoot.tsx";
import { WorkHead } from "./WorkHead.tsx";

/**
 * One piece of agent work on the issue (UX §4.17): who works and where it stands, the brief, each
 * attempt, the questions answered right there, *Approve plan*, the review, and why the agent gave
 * up. It is a record of its own, apart from the comments.
 */
export function WorkPiece({
  work,
  issue,
  statusChanges,
}: {
  readonly work: Work;
  readonly issue: string;
  readonly statusChanges: readonly Schema<"StatusChangeRef">[] | null | undefined;
}) {
  return (
    <div {...partOf(part.agentWorkPiece)}>
      <WorkHead work={work} />
      <WorkBrief work={work} />
      {work.attempts.map((attempt) => (
        <AttemptLines key={attempt.id} work={work} attempt={attempt} />
      ))}
      <Questions work={work} />
      <WorkActions work={work} issue={issue} statusChanges={statusChanges} />
      <Reviews work={work} />
      <WorkFoot work={work} />
    </div>
  );
}
