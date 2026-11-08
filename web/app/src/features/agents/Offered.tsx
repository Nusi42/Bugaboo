import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { personName } from "../../shared/person.ts";
import { ActionError } from "./ActionError.tsx";
import { useAcceptAgent, useTakeBackOffer, type OfferedAgent } from "./agents.ts";

/** One agent handed to the person: who hands it over, *Accept* and *Decline*, each with its own request. */
function OfferedRow({ agent }: { readonly agent: OfferedAgent }) {
  const accept = useAcceptAgent();
  const decline = useTakeBackOffer();
  return (
    <li {...partOf(part.item)}>
      <span {...partOf(part.startColumn)}>
        <span {...partOf(part.itemTitle)}>{agent.name}</span>
        <span {...partOf(part.muted)}>
          {t("agents.offered-line", { person: personName(agent.from) })}
        </span>
      </span>
      <Button
        look={buttonLook.primary}
        isPending={accept.isPending}
        aria-label={t("agents.accept-name", { agent: agent.name })}
        onPress={() => {
          accept.mutate({ params: { path: { agent: agent.id } } });
        }}
      >
        {t("agents.accept")}
      </Button>
      <Button
        isPending={decline.isPending}
        aria-label={t("agents.decline-name", { agent: agent.name })}
        onPress={() => {
          decline.mutate({ params: { path: { agent: agent.id } } });
        }}
      >
        {t("agents.decline")}
      </Button>
      <ActionError error={accept.error ?? decline.error} />
    </li>
  );
}

/** The agents another person handed to this one. */
export function Offered({ offered }: { readonly offered: readonly OfferedAgent[] }) {
  return offered.length === 0 ? null : (
    <ul {...partOf(part.list)} aria-label={t("agents.offered")}>
      {offered.map((agent) => (
        <OfferedRow key={agent.id} agent={agent} />
      ))}
    </ul>
  );
}
