import { t } from "@bugaboo/messages";
import { Alert, alertTone, Form, SubmitButton } from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { ProjectNameField, ShortNameField } from "./ProjectFields.tsx";
import styles from "./Projects.module.css";
import { useMakeProject } from "./use-projects.ts";

export interface MakeProjectFormProps {
  /** The project it goes into (an ID); the top level without it. */
  readonly inside?: string;
}

/** *Make a project*: a name and a short name; at the top level, or inside the project given. */
export function MakeProjectForm({ inside }: MakeProjectFormProps) {
  const [name, setName] = useState("");
  const [shortName, setShortName] = useState("");
  const [made, setMade] = useState<string | null>(null);
  const make = useMakeProject(inside);
  const submit = (): void => {
    setMade(null);
    make.mutate(
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
      {make.isError ? <Alert tone={alertTone.danger}>{messageOf(make.error)}</Alert> : null}
      {made === null ? null : (
        <Alert tone={alertTone.success}>{t("projects.made", { name: made })}</Alert>
      )}
      <div className={styles.row}>
        <SubmitButton isPending={make.isPending}>{t("projects.make")}</SubmitButton>
      </div>
    </Form>
  );
}
