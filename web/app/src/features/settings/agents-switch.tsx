import { t } from "@bugaboo/messages";

import { ProjectAgentsSwitch } from "./ProjectAgentsSwitch.tsx";
import { type ControlRenderer } from "./SettingRow.tsx";

/**
 * The key of the setting that lets agents work: the organization has it, and a project can switch its agents off.
 * Written here because the generated list `settingKeys` holds only the settings of `settings/catalog.rs`, and this one is
 * declared in `settings/agents.rs` (the settings registry, ADR-0018).
 */
export const agentsEnabledKey = "agents.enabled";

/** The explanation under the switch, for a project: the organization's own help speaks of the whole organization. */
export function projectAgentsHelp(): string {
  return t("setting.agents.project-help");
}

/** *Agents in this project*, as the control of the setting in the settings of a project. */
export const projectAgentsControl: ControlRenderer = (setting, onChange, onReset) => (
  <ProjectAgentsSwitch
    value={{
      own: setting.own,
      on: setting.value !== false,
      from: { kind: setting.source.kind, project: setting.source.project?.short_name },
    }}
    onOff={() => {
      onChange(false);
    }}
    onOn={() => {
      onChange(true);
    }}
    onFollow={onReset}
  />
);
