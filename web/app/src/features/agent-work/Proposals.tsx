import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, partOf, part } from "@bugaboo/ui";
import { useId } from "react";

import { messageOf } from "../../shared/errors.ts";
import { fieldNameOf } from "../../shared/field-names.ts";
import { useDecideProposal, type Proposal } from "./agent-work.ts";
import { valueText } from "./value-text.ts";

/**
 * One proposal of a standing rule: what would change, with *Apply* and *Discard*. It changes
 * nothing until a person applies it, through the one path the assistant's proposals take.
 *
 * This is the seam for the proposal card of the assistant (WP-10): once that card is on `main`,
 * it is drawn here and this line goes.
 */
function ProposalLine({
  proposal,
  agent,
}: {
  readonly proposal: Proposal;
  readonly agent: string;
}) {
  const decide = useDecideProposal();
  const decideWith = (apply: boolean): void => {
    decide.mutate({ params: { path: { proposal: proposal.id } }, body: { apply } });
  };
  return (
    <li {...partOf(part.agentWorkQuestion)}>
      <p {...partOf(part.agentWorkWords)}>
        {t("agent-work.proposal-line", {
          agent,
          field: fieldNameOf({ code: proposal.field }),
          before: valueText(proposal.before),
          after: valueText(proposal.after),
        })}
      </p>
      {decide.isError ? <Alert tone={alertTone.danger}>{messageOf(decide.error)}</Alert> : null}
      <div {...partOf(part.agentWorkActions)}>
        <Button
          look={buttonLook.primary}
          isPending={decide.isPending}
          onPress={() => {
            decideWith(true);
          }}
        >
          {t("agent-work.proposal-apply")}
        </Button>
        <Button
          isPending={decide.isPending}
          onPress={() => {
            decideWith(false);
          }}
        >
          {t("agent-work.proposal-discard")}
        </Button>
      </div>
    </li>
  );
}

/** The proposals of standing rules that wait for a person. Nothing to show leaves no heading. */
export function Proposals({
  proposals,
  agents,
}: {
  readonly proposals: readonly Proposal[];
  /** The names of the agents by their ID. */
  readonly agents: Readonly<Record<string, string>>;
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
          <ProposalLine
            key={proposal.id}
            proposal={proposal}
            agent={agents[proposal.agent] ?? ""}
          />
        ))}
      </ul>
    </section>
  );
}
