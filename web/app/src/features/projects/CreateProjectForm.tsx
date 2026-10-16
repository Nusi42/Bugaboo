import { t } from "@bugaboo/messages";
import { Alert, alertTone, Form, SubmitButton, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { ProjectNameField, ShortNameField } from "./ProjectFields.tsx";
import { useCreateProject } from "./use-projects.ts";

export interface CreateProjectFormProps {
  /** The project it goes into (an ID); the top level without it. */
  readonly inside?: string;
}

/** *Create a project*: a name and a short name; at the top level, or inside the project given. */
export function CreateProjectForm({ inside }: CreateProjectFormProps) {
  const [name, setName] = useState("");
  const [shortName, setShortName] = useState("");
  const [made, setMade] = useState<string | null>(null);
  const create = useCreateProject(inside);
  const submit = (): void => {
    setMade(null);
    create.mutate(
      { shortName: shortName.trim(), name: name.trim() },
      {
        onSuccess: (project) => {
          setName("");
          setShortName("");
          setMade(project.name);
        },
      },
    );
  };
  return (
    <Form onSubmit={submit}>
      <ProjectNameField value={name} onChange={setName} />
      <ShortNameField value={shortName} onChange={setShortName} />
      {create.isError ? <Alert tone={alertTone.danger}>{messageOf(create.error)}</Alert> : null}
      {made === null ? null : (
        <Alert tone={alertTone.success}>{t("projects.made", { name: made })}</Alert>
      )}
      <div {...partOf(part.row)}>
        <SubmitButton isPending={create.isPending}>{t("projects.create")}</SubmitButton>
      </div>
    </Form>
  );
}
