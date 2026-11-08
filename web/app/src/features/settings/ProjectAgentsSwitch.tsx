import { t } from "@bugaboo/messages";
import { SelectField } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { type Setting } from "../../shared/my-settings.ts";
import { agentsEnabledKey } from "./agents-enabled-key.ts";
import { organizationOptions } from "./organization.ts";

/** What the project chose: it follows the organization, or its agents are off. */
const choice = { organization: "organization", off: "off" } as const;

/** The value of the organization, if the person may read it; else the value of the project, which counts when it has none of its own. */
function useOrganizationValue(here: boolean | undefined): boolean | undefined {
  const organization = useQuery(organizationOptions(agentsEnabledKey)).data?.settings.find(
    (found: Setting) => found.key === agentsEnabledKey,
  )?.value;
  return typeof organization === "boolean" ? organization : here;
}

export interface ProjectAgentsSwitchProps {
  /** The project switched its agents off. */
  readonly off: boolean;
  /** The value that counts in a project that has none of its own: the organization's; none for a project that has one. */
  readonly here: boolean | undefined;
  readonly onOff: () => void;
  /** Back to the value of the organization. */
  readonly onFollow: () => void;
}

/** *Agents in this project* (UX §5.1): *As the organization* (with its value) or *Off*. */
export function ProjectAgentsSwitch({ off, here, onOff, onFollow }: ProjectAgentsSwitchProps) {
  const organization = useOrganizationValue(here);
  const follows =
    organization === undefined
      ? t("setting.agents.project-follows")
      : t(
          organization ? "setting.agents.project-follows-on" : "setting.agents.project-follows-off",
        );
  return (
    <SelectField
      label={t("setting.agents.project-switch")}
      value={off ? choice.off : choice.organization}
      onChange={(picked) => {
        if (picked === choice.off) {
          onOff();
        } else {
          onFollow();
        }
      }}
      choices={[
        { value: choice.organization, label: follows },
        { value: choice.off, label: t("setting.agents.project-off") },
      ]}
    />
  );
}
