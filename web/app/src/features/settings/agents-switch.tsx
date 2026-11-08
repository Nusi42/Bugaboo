import { t } from "@bugaboo/messages";

import { ProjectAgentsSwitch } from "./ProjectAgentsSwitch.tsx";
import { type ControlRenderer } from "./SettingRow.tsx";

/** The explanation under the switch, for a project: the organization's own help speaks of the whole organization. */
export function projectAgentsHelp(): string {
  return t("setting.agents.project-help");
}

/** *Agents in this project*, as the control of the setting in the settings of a project. */
export const projectAgentsControl: ControlRenderer = (setting, onChange, onReset) => (
  <ProjectAgentsSwitch
    off={setting.own && setting.value === false}
    here={!setting.own && typeof setting.value === "boolean" ? setting.value : undefined}
    onOff={() => {
      onChange(false);
    }}
    onFollow={onReset}
  />
);
