import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { NameValueList } from "@bugaboo/ui";

import { FieldLine } from "./FieldLine.tsx";
import { lineKey } from "./line-key.ts";
import { ProjectFieldsCard } from "./project-fields.tsx";

type Project = Schema<"ProjectDetailBody">;

/**
 * The own fields of a project on its overview (CF-08): the same fields with the scope *Projects*,
 * in their look, and an editor for the person who may change the project.
 */
export function OwnFieldsOfProject({ project }: { readonly project: Project }) {
  return (
    <ProjectFieldsCard
      project={project}
      heading={t("fields.project-heading")}
      wanted={(field) => field.applies_to === "projects"}
    >
      {(shown, { mayChange, valueOf, commit }) => (
        <NameValueList>
          {shown.map((field) => (
            <FieldLine
              key={lineKey(field, valueOf(field), project.project.version)}
              field={field}
              project={project.project.short_name}
              value={valueOf(field)}
              editable={mayChange(field)}
              onCommit={commit}
            />
          ))}
        </NameValueList>
      )}
    </ProjectFieldsCard>
  );
}
