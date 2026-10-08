import { t } from "@bugaboo/messages";
import { Link, partOf, part } from "@bugaboo/ui";

import { projectAddress } from "../../shared/paths.ts";
import { projectPageAddress, projectPages } from "../../shared/project-pages.ts";
import { QueryGate } from "../shell/index.ts";
import { ChangeProjectForm } from "./ChangeProjectForm.tsx";
import { useProject, type OneProject } from "./use-projects.ts";

/** The editor of one project in the side panel: its name and short name, and the way to its pages. */
export function ProjectEditor({ shortName }: { readonly shortName: string }) {
  const project = useProject(shortName);
  const found: OneProject | undefined = project.data;
  return (
    <QueryGate error={project.error} data={found}>
      {(shown) => (
        <>
          {shown.permissions.includes("change-project-settings") ? (
            <ChangeProjectForm key={shown.project.id} project={shown} />
          ) : null}
          <div {...partOf(part.row)}>
            <Link href={projectAddress(shortName)}>{t("projects.open")}</Link>
            <Link href={projectPageAddress(projectPages.settings, shortName)}>
              {t("projects.settings-link")}
            </Link>
          </div>
        </>
      )}
    </QueryGate>
  );
}
