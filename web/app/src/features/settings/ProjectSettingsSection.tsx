import { QueryGate } from "../shell/index.ts";
import { useProjectSettings } from "./project.ts";
import { SettingsForm } from "./SettingsForm.tsx";

export interface ProjectSettingsSectionProps {
  /** The ID of the project. */
  readonly projectId: string;
}

/**
 * The settings of one project (PRJ-02, CFG-06), drawn by the generic form: the section a project's
 * page shows for the people who may change them. It draws nothing while no setting can be set on a
 * project (M2 has none yet), so the project page can hold it already.
 */
export function ProjectSettingsSection({ projectId }: ProjectSettingsSectionProps) {
  const { settings, failed, edits } = useProjectSettings(projectId);
  return (
    <QueryGate error={failed} data={settings} loading={null}>
      {(shown) =>
        shown.settings.length === 0 ? null : (
          <SettingsForm
            settings={shown.settings}
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
        )
      }
    </QueryGate>
  );
}
