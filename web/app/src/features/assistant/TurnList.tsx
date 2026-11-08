import { t } from "@bugaboo/messages";
import { Announce, MarkdownView, partOf, part } from "@bugaboo/ui";

import { AssistantProposal } from "./AssistantProposal.tsx";
import type { Turn } from "./conversation.ts";

/** One turn: what the person asked, or what the assistant answered (labelled, with its cards). */
function OneTurn({ turn }: { readonly turn: Turn }) {
  if (turn.from === "person") {
    return (
      <li {...partOf(part.assistantTurn, { variants: ["person"] })}>
        <span {...partOf(part.muted)}>{t("assistant.you")}</span>
        <p {...partOf(part.assistantQuestion)}>{turn.text}</p>
      </li>
    );
  }
  return (
    <li {...partOf(part.assistantTurn, { variants: ["assistant"] })}>
      <span {...partOf(part.muted)}>{t("assistant.label")}</span>
      <MarkdownView html={turn.answer.html} />
      {turn.answer.proposals.map((proposal) => (
        <AssistantProposal key={proposal.reference} proposal={proposal} />
      ))}
    </li>
  );
}

/**
 * The turns of the conversation. A screen reader is told when an answer has arrived, and the list
 * itself stays quiet, so a long answer is not read twice.
 */
export function TurnList({
  turns,
  pending,
}: {
  readonly turns: readonly Turn[];
  /** The question that waits for its answer. */
  readonly pending?: string | undefined;
}) {
  const answers = turns.filter((turn) => turn.from === "assistant").length;
  return (
    <>
      {turns.length === 0 && pending === undefined ? (
        <p {...partOf(part.muted)}>{t("assistant.empty")}</p>
      ) : (
        <ol {...partOf(part.assistantTurns)} aria-label={t("assistant.conversation")}>
          {turns.map((turn) => (
            <OneTurn key={turn.id} turn={turn} />
          ))}
          {pending === undefined ? null : (
            <OneTurn turn={{ id: turns.length, from: "person", text: pending }} />
          )}
        </ol>
      )}
      <Announce>{answers === 0 ? "" : t("assistant.answered", { count: answers })}</Announce>
    </>
  );
}
