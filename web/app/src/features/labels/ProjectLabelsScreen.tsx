import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, partOf, part } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { projectPages } from "../../shared/project-pages.ts";
import { offerChangeBody, ProjectOffers, ProjectSettingsScreen } from "../projects/index.ts";
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
  const change = api.query.useMutation("put", "/api/v1/label-groups/{label_group}/offers", {
    onSuccess: refreshLabels,
  });
  return (
    <ProjectOffers
      label={t("labels.project-list")}
      offers={groups.map((group) => ({
        id: group.id,
        name: group.name,
        beside: group.retired ? (
          <span {...partOf(part.muted)}>{t("labels.retired")}</span>
        ) : undefined,
        offered: group.offered_here,
        own: group.own_offer,
        mayChange: group.may_change_offer,
      }))}
      isPending={change.isPending}
      error={change.error}
      onChange={(id, offered) => {
        const group = groups.find((each) => each.id === id);
        if (group !== undefined) {
          change.mutate({
            params: { path: { label_group: id } },
            body: offerChangeBody(group.version, project, offered),
          });
        }
      }}
    />
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
    <ProjectSettingsScreen address={address} page={projectPages.labels}>
      {(project) => (
        <ProjectLabels project={project.project.short_name} name={project.project.name} />
      )}
    </ProjectSettingsScreen>
  );
}
