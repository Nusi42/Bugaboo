import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, partOf, part } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { projectPages } from "../../shared/project-pages.ts";
import { OfferedHere, ProjectGate, ProjectTabs } from "../projects/index.ts";
import { CreateLabelForm } from "./CreateLabelForm.tsx";
import { CreateLabelGroupForm } from "./CreateLabelGroupForm.tsx";
import { refreshLabels, useLabels, type LabelGroup } from "./labels.ts";

/** The groups of labels, each with a switch that says whether the project offers it. */
function Groups({
  project,
  groups,
}: {
  readonly project: string;
  readonly groups: readonly LabelGroup[];
}) {
  const change = api.query.useMutation("put", "/api/v1/label-groups/{group}/offers", {
    onSuccess: refreshLabels,
  });
  return (
    <div {...partOf(part.stack)}>
      <ul {...partOf(part.list)} aria-label={t("labels.project-list")}>
        {groups.map((group) => (
          <OfferedHere
            key={group.id}
            name={group.name}
            beside={
              group.retired ? <span {...partOf(part.muted)}>{t("labels.retired")}</span> : undefined
            }
            offered={group.offered_here ?? true}
            own={group.own_offer ?? undefined}
            mayChange={group.may_change_offer === true}
            isPending={change.isPending}
            onChange={(offered) => {
              change.mutate({
                params: { path: { group: group.id } },
                body: {
                  version: group.version,
                  changes: [{ project, ...(offered === null ? {} : { offered }) }],
                },
              });
            }}
          />
        ))}
      </ul>
      {change.isError ? <Alert tone={alertTone.danger}>{messageOf(change.error)}</Alert> : null}
    </div>
  );
}

/** The cards of the screen once the project is known. */
function ProjectLabels({ project, name }: { readonly project: string; readonly name: string }) {
  const labels = useLabels(project);
  const groups = labels.groups;
  return (
    <>
      <Card heading={t("labels.project-title", { project: name })} level={1}>
        <p {...partOf(part.text)}>{t("labels.project-intro")}</p>
        {labels.isError ? <Alert tone={alertTone.danger}>{messageOf(labels.error)}</Alert> : null}
        <Groups project={project} groups={groups} />
      </Card>
      <Card heading={t("labels.project-create-title")}>
        <p {...partOf(part.text)}>{t("labels.project-create-intro")}</p>
        <CreateLabelGroupForm project={project} />
      </Card>
      <Card heading={t("labels.project-create-label-title")}>
        <p {...partOf(part.text)}>{t("labels.project-create-label-intro")}</p>
        <CreateLabelForm groups={groups} inGroupsOnly />
      </Card>
    </>
  );
}

/**
 * `/p/WEB/settings/labels`: the groups of labels a project offers (TAG-02, M3-F122), below the tabs
 * of its settings, and the forms that create a group and a label for this project. A Project lead
 * looks after these; a group of the whole organization can be switched off here, and only here.
 */
export function ProjectLabelsScreen({ address }: { readonly address: string }) {
  return (
    <ProjectGate address={address} page={projectPages.labels}>
      {(project) => (
        <div {...partOf(part.stack)}>
          <ProjectTabs project={project} />
          <ProjectLabels project={project.project.short_name} name={project.project.name} />
        </div>
      )}
    </ProjectGate>
  );
}
