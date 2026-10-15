import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  Card,
  Form,
  SubmitButton,
  Switch,
  partOf,
  part,
} from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { listText } from "../../shared/list-text.ts";
import { QueryGate } from "../shell/index.ts";
import { useFieldUsage, type OwnField } from "./fields.ts";
import { IssueTypeBoxes } from "./IssueTypeBoxes.tsx";
import { ProjectsToUse, type ProjectChoice } from "./ProjectsToUse.tsx";
import { fieldChange, useChangeField } from "./use-change-field.ts";

/** A project in the list of the scope, with the button that takes it out. */
function ProjectLine({
  project,
  onRemove,
}: {
  readonly project: ProjectChoice;
  readonly onRemove: () => void;
}) {
  return (
    <li {...partOf(part.item)}>
      <span>{project.name}</span>
      <Button
        look={buttonLook.quiet}
        aria-label={t("fields.scope-remove", { name: project.name })}
        onPress={onRemove}
      >
        {t("fields.scope-remove-button")}
      </Button>
    </li>
  );
}

/** What the field is used for now: the projects with a value and the issue types, before a change is made. */
function WhereUsed({ field }: { readonly field: OwnField }) {
  const usage = useFieldUsage(field.id);
  return (
    <QueryGate error={usage.error} data={usage.data}>
      {(found) => (
        <div {...partOf(part.stack)}>
          <p {...partOf(part.text)}>
            {found.whole_organization
              ? t("fields.used-everywhere", { count: found.project_count })
              : t("fields.used-in-projects", { count: found.project_count })}
          </p>
          {found.projects.length === 0 ? null : (
            <ul {...partOf(part.list)} aria-label={t("fields.used-list")}>
              {found.projects.map((project) => (
                <li key={project.id} {...partOf(part.item)}>
                  {project.name}
                </li>
              ))}
            </ul>
          )}
          {found.truncated ? (
            <p {...partOf(part.muted)}>
              {t("fields.used-more", { count: found.project_count - found.projects.length })}
            </p>
          ) : null}
          {field.scope.issue_types.length === 0 ? null : (
            <p {...partOf(part.muted)}>
              {t("fields.used-types", {
                names: listText(field.scope.issue_types.map((each) => each.name)),
              })}
            </p>
          )}
        </div>
      )}
    </QueryGate>
  );
}

/** The projects a field is limited to: the list, and the picker that adds one. */
function ProjectScope({
  projects,
  setProjects,
}: {
  readonly projects: readonly ProjectChoice[];
  readonly setProjects: (projects: readonly ProjectChoice[]) => void;
}) {
  return (
    <div {...partOf(part.stack)}>
      <ul {...partOf(part.list)} aria-label={t("fields.scope-projects-list")}>
        {projects.map((project) => (
          <ProjectLine
            key={project.id}
            project={project}
            onRemove={() => {
              setProjects(projects.filter((each) => each.id !== project.id));
            }}
          />
        ))}
      </ul>
      <ProjectsToUse
        picked={projects.map((each) => each.id)}
        onPick={(project) => {
          setProjects([...projects, project]);
        }}
      />
    </div>
  );
}

/**
 * *Where it's used* of the field page (CF-03, task 6): the projects and issue types that use the
 * field now, listed before a change, and the form that limits the field to some projects (with their
 * sub-projects) and some issue types.
 */
export function UsageSection({ field }: { readonly field: OwnField }) {
  const [everywhere, setEverywhere] = useState(field.scope.whole_organization);
  const [projects, setProjects] = useState<readonly ProjectChoice[]>(
    field.scope.projects.map((project) => ({
      id: project.id,
      name: project.name,
      shortName: project.short_name,
    })),
  );
  const [types, setTypes] = useState<readonly string[]>(
    field.scope.issue_types.map((each) => each.id),
  );
  const change = useChangeField();
  return (
    <Card heading={t("fields.usage")}>
      <WhereUsed field={field} />
      <Form
        onSubmit={() => {
          change.mutate(
            fieldChange(field, {
              scope: {
                projects: everywhere ? [] : projects.map((each) => each.id),
                issue_types: types,
              },
            }),
          );
        }}
      >
        <Switch
          label={t("fields.scope-everywhere-switch")}
          isSelected={everywhere}
          onChange={setEverywhere}
        />
        {everywhere ? null : <ProjectScope projects={projects} setProjects={setProjects} />}
        {field.applies_to === "issues" ? (
          <IssueTypeBoxes types={types} setTypes={setTypes} />
        ) : null}
        {change.isError ? <Alert tone={alertTone.danger}>{messageOf(change.error)}</Alert> : null}
        <div {...partOf(part.row)}>
          <SubmitButton isPending={change.isPending}>{t("common.save")}</SubmitButton>
        </div>
      </Form>
    </Card>
  );
}
