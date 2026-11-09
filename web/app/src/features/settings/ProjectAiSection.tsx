import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { QueryGate } from "../shell/index.ts";
import { SwitchBox } from "./ControlWithHint.tsx";
import { projectShows } from "./project-kinds.ts";
import { ProjectSettingsSection } from "./ProjectSettingsSection.tsx";
import { useChangeProjectAi, useMarkProject, useNeverSentMarks, useProjectAi } from "./ai.ts";

/** The switch *Assistant in this project*; the project follows the one of the organization until it has its own. */
function ProjectSwitch({ project }: { readonly project: string }) {
  const found = useProjectAi(project);
  const change = useChangeProjectAi();
  return (
    <QueryGate error={found.error} data={found.data} loading={null}>
      {(shown) => (
        <SwitchBox
          label={t("ai-setup.project-switch")}
          isSelected={shown.works_here}
          onChange={(enabled) => {
            change.mutate({ params: { path: { project } }, body: { enabled } });
          }}
          help={
            shown.organization_on
              ? t("ai-setup.project-help")
              : t("ai-setup.project-organization-off")
          }
        >
          {shown.own === undefined || shown.own === null ? null : (
            <div {...partOf(part.row)}>
              <Button
                look={buttonLook.quiet}
                onPress={() => {
                  change.mutate({ params: { path: { project } }, body: {} });
                }}
              >
                {t("ai-setup.project-follow")}
              </Button>
            </div>
          )}
          {change.isError ? <Alert tone={alertTone.danger}>{messageOf(change.error)}</Alert> : null}
        </SwitchBox>
      )}
    </QueryGate>
  );
}

/** The switch *Never sent to AI* of the project, with every project inside it. */
function ProjectMark({ project }: { readonly project: string }) {
  const marks = useNeverSentMarks();
  const mark = useMarkProject();
  const found = marks.data?.projects.find((each) => each.project.short_name === project);
  return (
    <SwitchBox
      label={t("ai-setup.never-sent")}
      isSelected={found?.marked === true}
      onChange={(marked) => {
        mark.mutate({
          params: { path: { project } },
          body: { marked, version: found?.version ?? 0 },
        });
      }}
      help={t("ai-setup.never-sent-project-help")}
    >
      {mark.isError ? <Alert tone={alertTone.danger}>{messageOf(mark.error)}</Alert> : null}
    </SwitchBox>
  );
}

/**
 * The AI part of a project's settings (AI-02, AI-03, WP-15): one switch for each of the three kinds
 * of AI, in the order agents, AI apps, the assistant last (each follows the organization's until the
 * project has its own, and none switches another off), and *Never sent to AI*, which holds for all
 * three even for people who can read the project.
 */
export function ProjectAiSection({
  project,
  projectId,
}: {
  readonly project: string;
  readonly projectId: string;
}) {
  return (
    <div {...partOf(part.stack)}>
      <ProjectSettingsSection projectId={projectId} show={projectShows.kindsOfAi} />
      <ProjectSwitch project={project} />
      <ProjectMark project={project} />
    </div>
  );
}
