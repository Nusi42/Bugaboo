import { t } from "@bugaboo/messages";
import { Link, partOf, part, SelectField } from "@bugaboo/ui";

import { useMyAgents } from "./agent-work.ts";
import type { DraftProps } from "./hand-over-draft.ts";
import { paths } from "../../shared/paths.ts";

/** The agents of the person that can work now, as the choices of the window. */
function useAgentChoices() {
  const { agents } = useMyAgents();
  return agents.flatMap((agent) =>
    agent.state === "active" && typeof agent.id === "string" && typeof agent.name === "string"
      ? [{ value: agent.id, label: agent.name }]
      : [],
  );
}

/** The agent to hand over to, or the way to create one when the person has none (only their own agents are offered). */
export function AgentChoice({ draft, onChange }: DraftProps) {
  const choices = useAgentChoices();
  if (choices.length === 0) {
    return (
      <>
        <p {...partOf(part.text)}>{t("agent-work.agent-none")}</p>
        <p {...partOf(part.text)}>
          <Link href={paths.myAgents}>{t("agent-work.agent-create")}</Link>
        </p>
      </>
    );
  }
  return (
    <SelectField
      label={t("agent-work.agent")}
      value={draft.agent}
      onChange={(agent) => {
        onChange({ agent });
      }}
      choices={[{ value: "", label: "" }, ...choices]}
    />
  );
}
