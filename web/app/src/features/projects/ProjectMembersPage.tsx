import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, partOf, part, ShowMoreOf } from "@bugaboo/ui";

import { itemsOf } from "../../shared/paged.ts";
import { mayOpenProjectPage, projectPages } from "../../shared/project-pages.ts";
import { NotFoundPage, QueryGate } from "../shell/index.ts";
import { MemberRow } from "./MemberRow.tsx";
import { ProjectGate } from "./ProjectGate.tsx";
import { ProjectSettingsFrame } from "./ProjectSettingsFrame.tsx";
import { useProjectShares, useShareChanges, type MemberShare } from "./shares.ts";
import type { OneProject } from "./use-projects.ts";

/** The page once the project is known: the tabs, then the people with their shares. */
function Members({ project }: { readonly project: OneProject }) {
  const id = project.project.id;
  const shares = useProjectShares(id);
  const changes = useShareChanges(id);
  const members: readonly MemberShare[] = itemsOf(shares.data?.pages);
  if (!mayOpenProjectPage(projectPages.members, project)) {
    return <NotFoundPage />;
  }
  return (
    <ProjectSettingsFrame project={project}>
      <Card heading={t("nav.project-members")} level={1} intro={t("project-members.intro")}>
        <QueryGate error={shares.error} data={shares.data === undefined ? undefined : true}>
          {() =>
            members.length === 0 ? (
              <p {...partOf(part.muted)}>{t("project-members.none")}</p>
            ) : (
              <>
                {changes.problem === null ? null : (
                  <Alert tone={alertTone.danger}>{changes.problem.message}</Alert>
                )}
                <ul
                  {...partOf(part.list, { variants: ["snug"] })}
                  aria-label={t("nav.project-members")}
                >
                  {members.map((member, at) => (
                    <MemberRow
                      key={member.person.shown === "name" ? member.person.id : at}
                      member={member}
                      changes={changes}
                    />
                  ))}
                </ul>
                <ShowMoreOf label={t("project-members.more")} list={shares} />
              </>
            )
          }
        </QueryGate>
      </Card>
    </ProjectSettingsFrame>
  );
}

/**
 * *Project settings › Project members*, `/p/WEB/settings/members` (PLAN-15, UX §4.13): every person
 * with a role in the project, directly or through a group, with their *Share in the project*, which
 * a person who decides who has which role there sets in place, with *Undo*. Its people are the ones
 * *Who has which role* gives a role here, as the server lists them; to everybody else the page is not
 * found.
 */
export function ProjectMembersPage({ address }: { readonly address: string }) {
  return (
    <ProjectGate address={address} page={projectPages.members}>
      {(project) => <Members project={project} />}
    </ProjectGate>
  );
}
