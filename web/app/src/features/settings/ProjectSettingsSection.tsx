import { Alert, alertTone } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
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
  if (failed !== null) {
    return <Alert tone={alertTone.danger}>{messageOf(failed)}</Alert>;
  }
  if (settings === undefined || settings.settings.length === 0) {
    return null;
  }
  return (
    <SettingsForm
      settings={settings.settings}
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
}
