import { t } from "@bugaboo/messages";
import { SelectField } from "@bugaboo/ui";

import {
  agentsChoice,
  projectAgentsChoice,
  type ProjectAgentsValue,
} from "./project-agents-choice.ts";

export interface ProjectAgentsSwitchProps {
  readonly value: ProjectAgentsValue;
  /** The project switches its agents off. */
  readonly onOff: () => void;
  /** The project switches its agents on, whatever the level above says. */
  readonly onOn: () => void;
  /** Back to the value of the level above. */
  readonly onFollow: () => void;
}

/** *Agents in this project* (UX §5.1): *As the organization* (or the project above, with its value), *Off*, and *On* where the project chose it. */
export function ProjectAgentsSwitch({ value, onOff, onOn, onFollow }: ProjectAgentsSwitchProps) {
  const { selected, follows, hasOn } = projectAgentsChoice(value);
  const pick = {
    [agentsChoice.follow]: onFollow,
    [agentsChoice.off]: onOff,
    [agentsChoice.on]: onOn,
  };
  return (
    <SelectField
      label={t("setting.agents.project-switch")}
      value={selected}
      onChange={(picked) => {
        pick[picked]();
      }}
      choices={[
        { value: agentsChoice.follow, label: follows },
        ...(hasOn ? [{ value: agentsChoice.on, label: t("setting.agents.project-on") }] : []),
        { value: agentsChoice.off, label: t("setting.agents.project-off") },
      ]}
    />
  );
}
