import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";
import { useId } from "react";

import { messageOf } from "../../shared/errors.ts";
import { ProposalCard, type CardState } from "../assistant/index.ts";
import { useDecideProposal, type Proposal } from "./agent-work.ts";

/** Where a proposal of a standing rule stands: waiting, being saved, or refused with the reason. */
function stateOf(isPending: boolean, failure: { readonly error: unknown } | null): CardState {
  if (isPending) {
    return { kind: "saving" };
  }
  return failure === null
    ? { kind: "open" }
    : { kind: "refused", problem: messageOf(failure.error) };
}

/**
 * One proposal of a standing rule, drawn as the *Proposed change* card of the assistant (UX §4.12):
 * the one look for a change that waits for a person. It changes nothing until a person applies it,
 * through the one path of proposals, and says which agent it comes from.
 */
function RuleProposal({
  proposal,
  issue,
}: {
  readonly proposal: Proposal;
  readonly issue: string;
}) {
  const decide = useDecideProposal();
  const decideWith = (apply: boolean): void => {
    decide.mutate({ params: { path: { proposal: proposal.id } }, body: { apply } });
  };
  return (
    <li>
      <ProposalCard
        issue={issue}
        changes={[{ field: proposal.field, before: proposal.before, after: proposal.after }]}
        state={stateOf(decide.isPending, decide.isError ? { error: decide.error } : null)}
        source={
          <span {...partOf(part.muted)}>
            {t("agent-work.proposal-from", { agent: proposal.agent_name })}
          </span>
        }
        onApply={() => {
          decideWith(true);
        }}
        onDiscard={() => {
          decideWith(false);
        }}
      />
    </li>
  );
}

/** The proposals of standing rules that wait for a person. Nothing to show leaves no heading. */
export function Proposals({
  proposals,
  issue,
}: {
  readonly proposals: readonly Proposal[];
  /** The readable ID of the issue they are about. */
  readonly issue: string;
}) {
  const heading = useId();
  if (proposals.length === 0) {
    return null;
  }
  return (
    <section {...partOf(part.agentWorkSection)} aria-labelledby={heading}>
      <h3 id={heading} {...partOf(part.agentWorkLabel)}>
        {t("agent-work.proposals")}
      </h3>
      <ul {...partOf(part.list)}>
        {proposals.map((proposal) => (
          <RuleProposal key={proposal.id} proposal={proposal} issue={issue} />
        ))}
      </ul>
    </section>
  );
}
