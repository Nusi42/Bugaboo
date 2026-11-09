import { t } from "@bugaboo/messages";
import { SelectField } from "@bugaboo/ui";

import { listedAgents, useMyAgentList } from "./agents.ts";
import { projectKey } from "./project-choices.ts";
import { ProjectSelect } from "./ProjectSelect.tsx";
import { type DraftProps } from "./rule-draft.ts";

/** The agents of the person that can work, as the choices of the window. */
function AgentSelect({ draft, onChange }: DraftProps) {
  const agents = listedAgents(useMyAgentList().data?.items).filter(
    (agent) => agent.state === "active",
  );
  return (
    <SelectField
      label={t("agents.rule-agent")}
      value={draft.agent}
      onChange={(agent) => {
        onChange({ agent });
      }}
      choices={[
        { value: "", label: t("agents.rule-choose-agent") },
        ...agents.map((agent) => ({ value: agent.id, label: agent.name })),
      ]}
    />
  );
}

/** Who works on what the rule finds, and in which project. */
export function RuleWhere({ draft, onChange }: DraftProps) {
  return (
    <>
      <AgentSelect draft={draft} onChange={onChange} />
      <ProjectSelect
        label={t("agents.rule-project")}
        valueKey={projectKey.id}
        value={draft.project}
        onChange={(project) => {
          onChange({ project });
        }}
      />
    </>
  );
}
