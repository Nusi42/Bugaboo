import { t } from "@bugaboo/messages";
import { Alert, Card } from "@bugaboo/ui";
import { useState } from "react";

import { AssignmentList } from "./AssignmentList.tsx";
import type { Subject } from "./assignments.ts";
import { GiveRoleForm } from "./GiveRoleForm.tsx";
import styles from "./Roles.module.css";
import { useRowChanges } from "./use-row-changes.ts";

export interface AssignmentsSectionProps {
  /** The project the rows are for, or `null` for the whole organization. */
  readonly project: { readonly id: string; readonly name: string } | null;
  /** The level of the heading: 1 when the section is the whole page, 2 inside a page. */
  readonly level?: 1 | 2;
}

/**
 * *Who has which role* (SEC-02, SEC-03): for the whole organization, or for one project with the rows
 * that come from above (each says where from and is changed for this project). Taking a row away
 * offers *Undo*; the form below gives a role or, on a project, *No access*.
 */
export function AssignmentsSection({ project, level = 2 }: AssignmentsSectionProps) {
  const id = project === null ? null : project.id;
  const changes = useRowChanges(id);
  const [preset, setPreset] = useState<Subject>();
  const here =
    project === null
      ? undefined
      : (from: { readonly who: Subject }) => {
          setPreset(from.who);
        };
  return (
    <>
      <Card
        heading={
          project === null
            ? t("assignments.title-organization")
            : t("assignments.title-project", { name: project.name })
        }
        level={level}
      >
        <p className={styles.text}>
          {t(project === null ? "assignments.intro-organization" : "assignments.intro-project")}
        </p>
        {changes.problem === null ? null : <Alert danger>{changes.problem}</Alert>}
        <AssignmentList project={id} onRemove={changes.remove} onChangeHere={here} />
      </Card>
      {changes.dialog}
      <Card heading={t("assignments.give-title")}>
        <GiveRoleForm
          key={preset === undefined ? "none" : `${preset.kind}:${preset.id}`}
          project={id}
          give={changes.give}
          preset={preset}
        />
      </Card>
    </>
  );
}
