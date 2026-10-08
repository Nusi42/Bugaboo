import { t } from "@bugaboo/messages";
import { Alert, alertTone, Form, SubmitButton, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { ProjectNameField, ShortNameField } from "./ProjectFields.tsx";
import { useChangeProject, type OneProject } from "./use-projects.ts";

export interface ChangeProjectFormProps {
  readonly project: OneProject;
  /** Called with the short name the project has after the change, when it was saved. */
  readonly onSaved?: (shortName: string) => void;
}

/**
 * The name and the short name of a project (PRJ-01, M2-Q4). The short name can be changed by
 * people who may make projects where this one sits; the old one stays with the project.
 */
export function ChangeProjectForm({ project, onSaved }: ChangeProjectFormProps) {
  const { id, name: oldName, short_name: oldShortName, version } = project.project;
  const mayChangeShortName = project.may_change_short_name;
  const [name, setName] = useState(oldName);
  const [shortName, setShortName] = useState(oldShortName);
  const [saved, setSaved] = useState(false);
  const change = useChangeProject();
  const submit = (): void => {
    setSaved(false);
    const [newName, newShortName] = [name.trim(), shortName.trim()];
    change.mutate(
      {
        id,
        version,
        ...(newName === oldName ? {} : { name: newName }),
        ...(!mayChangeShortName || newShortName === oldShortName
          ? {}
          : { shortName: newShortName }),
      },
      {
        onSuccess: (changed) => {
          setSaved(true);
          onSaved?.(changed.short_name);
        },
      },
    );
  };
  return (
    <Form onSubmit={submit}>
      <ProjectNameField value={name} onChange={setName} />
      {mayChangeShortName ? (
        <ShortNameField value={shortName} onChange={setShortName} changing />
      ) : (
        <p {...partOf(part.muted)}>{t("projects.short-name-fixed")}</p>
      )}
      {change.isError ? <Alert tone={alertTone.danger}>{messageOf(change.error)}</Alert> : null}
      {saved ? <Alert tone={alertTone.success}>{t("projects.saved")}</Alert> : null}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={change.isPending}>{t("common.save")}</SubmitButton>
      </div>
    </Form>
  );
}
