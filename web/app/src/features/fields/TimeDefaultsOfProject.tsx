import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { ProjectFieldsCard } from "./project-fields.tsx";
import { lineKey } from "./line-key.ts";
import { TimeDefaultLine } from "./TimeDefaultLine.tsx";

type Project = Schema<"ProjectDetailBody">;

/**
 * *Activity*, *Cost center* and the other fields of time entries as defaults of one project (TIME-13,
 * ADR-0035, decision 1): every entry booked on an issue of the project starts with them, unless the
 * issue sets its own; sub-projects inherit them (*Set by the project above*, said by the server from
 * the rule of the defaults) until they set their own (*Reset to default* takes it away). The lists
 * themselves are managed on *Fields*.
 */
export function TimeDefaultsOfProject({ project }: { readonly project: Project }) {
  return (
    <ProjectFieldsCard
      project={project}
      heading={t("fields.time-defaults-project")}
      intro={t("fields.time-defaults-project-intro")}
      wanted={(field, { mayChange }) => field.applies_to === "time-entries" && mayChange(field)}
    >
      {(shown, { valueOf, commit }) => (
        <ul {...partOf(part.list, { variants: ["loose"] })}>
          {shown.map((field) => (
            <TimeDefaultLine
              key={lineKey(field, valueOf(field), project.project.version)}
              field={field}
              project={project.project.short_name}
              own={valueOf(field)}
              inherited={project.inherited_fields[field.id]}
              source={project.inherited_from[field.id]?.project?.name}
              onCommit={commit}
            />
          ))}
        </ul>
      )}
    </ProjectFieldsCard>
  );
}
