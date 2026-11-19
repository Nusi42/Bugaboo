import { t } from "@bugaboo/messages";
import { Button, Checkbox, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { formatDateTime } from "../../shared/format.ts";
import { toggled } from "../../shared/toggled.ts";
import { ActionError } from "./ActionError.tsx";
import { AgentRow } from "./AgentRow.tsx";
import {
  approvedRequests,
  useAgentRequests,
  useDeclineAgentRequest,
  waitingRequests,
  type AgentRequest,
} from "./agent-requests.ts";
import { ApproveRequests } from "./ApproveRequests.tsx";
import { Section } from "./Section.tsx";

interface RequestRowProps {
  readonly request: AgentRequest;
  readonly ticked: boolean;
  readonly onTick: (on: boolean) => void;
}

/** One waiting request: who asked and why, the agents with what each is for, a tick and *Decline*. */
function RequestRow({ request, ticked, onTick }: RequestRowProps) {
  const decline = useDeclineAgentRequest();
  const address = request.web_address ?? null;
  return (
    <AgentRow>
      <Checkbox
        label={t("agents.request-tick", { app: request.asked_by })}
        hideLabel
        isSelected={ticked}
        onChange={onTick}
      />
      <span {...partOf(part.startColumn)}>
        <span {...partOf(part.itemTitle)}>
          {t("agents.request-line", { app: request.asked_by, count: request.agents.length })}
        </span>
        {request.by_app ? (
          <span {...partOf(part.muted)}>
            {address === null
              ? t("agents.request-app-computer")
              : t("agents.request-app-web", { address })}
          </span>
        ) : null}
        <span {...partOf(part.text)}>{request.reason}</span>
        <span {...partOf(part.muted)}>
          {request.agents
            .map((asked) => t("agents.request-agent", { name: asked.name, purpose: asked.purpose }))
            .join(" · ")}
        </span>
        <span {...partOf(part.muted)}>
          {t("agents.request-until", { moment: formatDateTime(request.waits_until) })}
        </span>
      </span>
      <Button
        isPending={decline.isPending}
        aria-label={t("agents.request-decline-name", { app: request.asked_by })}
        onPress={() => {
          decline.mutate({ params: { path: { request: request.id } } });
        }}
      >
        {t("agents.decline")}
      </Button>
      <ActionError error={decline.error} />
    </AgentRow>
  );
}

/** One approved request whose keys wait for the program that asked. */
function ApprovedRow({ request }: { readonly request: AgentRequest }) {
  return (
    <AgentRow>
      <span {...partOf(part.startColumn)}>
        <span {...partOf(part.itemTitle)}>
          {t("agents.request-approved", { app: request.asked_by, count: request.agents.length })}
        </span>
        <span {...partOf(part.muted)}>
          {t("agents.request-collect-until", { moment: formatDateTime(request.waits_until) })}
        </span>
      </span>
    </AgentRow>
  );
}

/**
 * *Asked for by a program* (ADR-0029, amendment M9): the requests that wait for the person, each
 * with *Decline*, and *Approve* for the ticked ones with the password once; then the approved
 * ones whose keys wait for the program. Nothing when there is none.
 */
export function AgentRequests() {
  const requests = useAgentRequests();
  const [ticked, setTicked] = useState<readonly string[]>([]);
  const items = requests.data?.requests ?? [];
  const waiting = waitingRequests(items);
  const approved = approvedRequests(items);
  if (waiting.length === 0 && approved.length === 0) {
    return null;
  }
  return (
    <Section heading={t("agents.requests")} count={waiting.length}>
      <p {...partOf(part.muted)}>{t("agents.requests-intro")}</p>
      <ul {...partOf(part.list)} aria-label={t("agents.requests")}>
        {waiting.map((request) => (
          <RequestRow
            key={request.id}
            request={request}
            ticked={ticked.includes(request.id)}
            onTick={(on) => {
              setTicked(toggled(ticked, request.id, on));
            }}
          />
        ))}
        {approved.map((request) => (
          <ApprovedRow key={request.id} request={request} />
        ))}
      </ul>
      {waiting.length === 0 ? null : (
        <ApproveRequests
          ticked={ticked.filter((id) => waiting.some((request) => request.id === id))}
          onApproved={() => {
            setTicked([]);
          }}
        />
      )}
    </Section>
  );
}
