import { t } from "@bugaboo/messages";

import { ProjectAgentsSwitch } from "./ProjectAgentsSwitch.tsx";
import { type ControlRenderer } from "./SettingRow.tsx";

/**
 * The key of the setting that lets agents work: the organization has it, and a project can switch its agents off.
 * Written here because the generated list `settingKeys` holds only the settings of `settings/catalog.rs`, and this one is
 * declared in `settings/agents.rs` (the settings registry, ADR-0018).
 */
export const agentsEnabledKey = "agents.enabled";

/** The key of the setting that lets AI apps work (`settings/ai_apps.rs`): the same switch, for the apps (WP-15). */
export const aiAppsEnabledKey = "ai-apps.enabled";

/** The explanation under the switch of agents, for a project: the organization's own help speaks of the whole organization. */
export function projectAgentsHelp(): string {
  return t("setting.agents.project-help");
}

/** The explanation under the switch of AI apps, for a project. */
export function projectAiAppsHelp(): string {
  return t("setting.ai-apps.project-help");
}

/** The switch of a kind of AI in the settings of a project, with the words of its kind for the label. */
function projectKindControl(label: () => string): ControlRenderer {
  return (setting, onChange, onReset) => (
    <ProjectAgentsSwitch
      label={label()}
      value={{
        own: setting.own,
        on: setting.value !== false,
        from: {
          kind: setting.source.kind,
          project: setting.source.project?.short_name,
        },
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
}

/** *Agents in this project*, as the control of the setting in the settings of a project. */
export const projectAgentsControl: ControlRenderer = projectKindControl(() =>
  t("setting.agents.project-switch"),
);

/** *AI apps in this project*, the same switch for the apps. */
export const projectAiAppsControl: ControlRenderer = projectKindControl(() =>
  t("setting.ai-apps.project-switch"),
);
