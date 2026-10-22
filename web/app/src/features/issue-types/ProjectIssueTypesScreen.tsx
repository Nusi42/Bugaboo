import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, IssueTypeIcon, partOf, part } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { projectPages } from "../../shared/project-pages.ts";
import { refreshIssueTypes } from "../../shared/query-keys.ts";
import { offerChangeBody, ProjectOffers, ProjectSettingsScreen } from "../projects/index.ts";
import { CreateIssueTypeForm } from "./CreateIssueTypeForm.tsx";
import { issueTypeLinesOf, useIssueTypes, type IssueTypeLine } from "./issue-types.ts";

/** What stands beside the name of a type: its icon and, while it is retired, a word that says so. */
function Beside({ issueType }: { readonly issueType: IssueTypeLine }) {
  return (
    <>
      <IssueTypeIcon name={issueType.icon} />
      {issueType.retired ? <span {...partOf(part.muted)}>{t("issue-types.retired")}</span> : null}
    </>
  );
}

/** The issue types, each with a switch that says whether the project offers it. */
function Types({ project }: { readonly project: string }) {
  const list = useIssueTypes(project);
  const change = api.query.useMutation("put", "/api/v1/issue-types/{issue_type}/offers", {
    onSuccess: refreshIssueTypes,
  });
  const types = issueTypeLinesOf(list.data?.pages);
  return (
    <div {...partOf(part.stack)}>
      {list.isError ? <Alert tone={alertTone.danger}>{messageOf(list.error)}</Alert> : null}
      <ProjectOffers
        label={t("issue-types.project-list")}
        offers={types.map((issueType) => ({
          id: issueType.id,
          name: issueType.name,
          beside: <Beside issueType={issueType} />,
          offered: issueType.here?.offered,
          own: issueType.here?.own,
          mayChange: issueType.here?.mayChange,
        }))}
        isPending={change.isPending}
        error={change.error}
        onChange={(id, offered) => {
          const issueType = types.find((each) => each.id === id);
          if (issueType !== undefined) {
            change.mutate({
              params: { path: { issue_type: id } },
              body: offerChangeBody(issueType.version, project, offered),
            });
          }
        }}
      />
    </div>
  );
}

/**
 * `/p/WEB/settings/issue-types`: the issue types a project offers (ISS-02, M3-F22), below the tabs
 * of its settings, and the form that creates a type for this project. A Project lead looks after
 * these; the types of the whole organization can be switched off here, and only here.
 */
export function ProjectIssueTypesScreen({ address }: { readonly address: string }) {
  return (
    <ProjectSettingsScreen address={address} page={projectPages.issueTypes}>
      {(project) => (
        <>
          <Card
            heading={t("issue-types.project-title", { project: project.project.name })}
            level={1}
          >
            <p {...partOf(part.text)}>{t("issue-types.project-intro")}</p>
            <Types project={project.project.short_name} />
          </Card>
          <Card heading={t("issue-types.project-create-title")}>
            <p {...partOf(part.text)}>{t("issue-types.project-create-intro")}</p>
            <CreateIssueTypeForm project={project.project.short_name} />
          </Card>
        </>
      )}
    </ProjectSettingsScreen>
  );
}
