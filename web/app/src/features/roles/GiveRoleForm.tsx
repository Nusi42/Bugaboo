import { t } from "@bugaboo/messages";
import { Alert, Button, ChoiceGroup, Form, SelectField } from "@bugaboo/ui";

import { useAllPages } from "../../shared/paged.ts";
import { type Subject, choicesOf, useChoices } from "./assignments.ts";
import { ChoicePicker } from "./ChoicePicker.tsx";
import styles from "./Roles.module.css";
import { NO_ACCESS, NOTHING, useGiveForm } from "./use-give-form.ts";
import type { RowSpec } from "./use-row-changes.ts";

export interface GiveRoleFormProps {
  /** The project the row is for (an ID), or `null` for the whole organization. */
  readonly project: string | null;
  /** Gives the row: rejects with the error of the server. */
  readonly give: (spec: RowSpec) => Promise<boolean>;
  /** Whom the form starts with, for *Change for this project* on a row that comes from above. */
  readonly preset?: Subject | undefined;
}

/** The roles to choose from, with *No access* last when the row is for a project. */
function useRoleChoices(project: string | null) {
  const roles = useChoices("role", project, "");
  // Every role the person may give, not the first page.
  useAllPages(roles);
  return [
    { value: NOTHING, label: t("assignments.choose-role") },
    ...choicesOf(roles.data?.pages).map((role) => ({ value: role.id, label: role.name })),
    ...(project === null ? [] : [{ value: NO_ACCESS, label: t("roles.no-access") }]),
  ];
}

/** Where it counts, for a row on a project: only the project, or it and its sub-projects. */
function Reach({
  value,
  onChange,
}: {
  readonly value: RowSpec["appliesTo"];
  readonly onChange: (value: RowSpec["appliesTo"]) => void;
}) {
  return (
    <ChoiceGroup
      legend={t("assignments.where")}
      choices={[
        { value: "project", label: t("roles.applies.project") },
        {
          value: "project-and-sub-projects",
          label: t("roles.applies.project-and-sub-projects"),
        },
      ]}
      value={value}
      onChange={onChange}
    />
  );
}

/**
 * *Give a role* (SEC-02): whom (a person or a group, found by name), the role, and, on a project,
 * where it counts. *No access* is a choice of the role, for a project only; it always covers the
 * project and its sub-projects, so the choice of reach is left out for it.
 */
export function GiveRoleForm({ project, give, preset }: GiveRoleFormProps) {
  const form = useGiveForm(give, preset);
  const roles = useRoleChoices(project);
  const noAccess = form.role === NO_ACCESS;
  return (
    <Form onSubmit={form.submit}>
      <ChoiceGroup
        legend={t("assignments.give-to")}
        choices={[
          { value: "person", label: t("assignments.kind-person") },
          { value: "group", label: t("assignments.kind-group") },
        ]}
        value={form.kind}
        onChange={form.setKind}
      />
      <ChoicePicker
        kind={form.kind}
        project={project}
        value={form.who?.id ?? null}
        onPick={form.pick}
      />
      {form.who === null ? null : (
        <p className={styles.text}>{t("assignments.picked", { name: form.who.name })}</p>
      )}
      <SelectField
        label={t("assignments.role")}
        choices={roles}
        value={form.role}
        onChange={form.setRole}
      />
      {project !== null && !noAccess ? <Reach value={form.reach} onChange={form.setReach} /> : null}
      {noAccess ? <p className={styles.muted}>{t("assignments.no-access-help")}</p> : null}
      {form.problem === null ? null : <Alert danger>{form.problem}</Alert>}
      <div className={styles.row}>
        <Button primary submit>
          {t(noAccess ? "assignments.give-no-access" : "assignments.give")}
        </Button>
      </div>
    </Form>
  );
}
