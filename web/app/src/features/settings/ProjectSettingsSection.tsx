import { QueryGate } from "../shell/index.ts";
import { useProjectSettings } from "./project.ts";
import {
  agentsEnabledKey,
  aiAppsEnabledKey,
  projectAgentsControl,
  projectAgentsHelp,
  projectAiAppsControl,
  projectAiAppsHelp,
} from "./agents-switch.tsx";
import { kindsOf, otherThanKinds, projectShows } from "./project-kinds.ts";
import { SettingsForm } from "./SettingsForm.tsx";

/** A project with more settings than this has a search box; a few settings do not need one. */
const SEARCH_FROM = 8;

export interface ProjectSettingsSectionProps {
  /** The ID of the project. */
  readonly projectId: string;
  /**
   * Which settings: the ones that are not the switches of a kind of AI (the default), or only those
   * switches, agents then AI apps (WP-15), for the card *AI in this project*.
   */
  readonly show?: (typeof projectShows)[keyof typeof projectShows];
}

/**
 * The settings of one project (PRJ-02, CFG-06), drawn by the generic form: the section a project's
 * page shows for the people who may change them. It draws nothing while no setting can be set on a
 * project (M2 has none yet), so the project page can hold it already.
 */
export function ProjectSettingsSection({
  projectId,
  show = projectShows.others,
}: ProjectSettingsSectionProps) {
  const { settings, failed, edits } = useProjectSettings(projectId);
  return (
    <QueryGate error={failed} data={settings} loading={null}>
      {(shown) => {
        const drawn =
          show === projectShows.kindsOfAi
            ? kindsOf(shown.settings)
            : otherThanKinds(shown.settings);
        return drawn.length === 0 ? null : (
          <SettingsForm
            settings={drawn}
            searchable={drawn.length > SEARCH_FROM}
            controls={{
              [agentsEnabledKey]: projectAgentsControl,
              [aiAppsEnabledKey]: projectAiAppsControl,
            }}
            helps={{
              [agentsEnabledKey]: projectAgentsHelp,
              [aiAppsEnabledKey]: projectAiAppsHelp,
            }}
            saveState={edits.saveState}
            problem={edits.problem}
            actions={{
              onChange: (key, value) => {
                edits.change({ kind: "change", key, value });
              },
              onReset: (key) => {
                edits.change({ kind: "reset", key });
              },
            }}
          />
        );
      }}
    </QueryGate>
  );
}
