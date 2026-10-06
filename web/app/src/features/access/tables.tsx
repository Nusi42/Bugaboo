import { t } from "@bugaboo/messages";

import { LevelTableView } from "./LevelTableView.tsx";
import type { LevelTable, PeopleKindKey, PeopleLevelCode } from "./levels.ts";
import { PeoplePreview } from "./PeoplePreview.tsx";
import { useFieldPermissions, usePeopleVisibility } from "./use-levels.ts";

/** The field permissions of a place: the whole organization, or a project (an ID). */
export function FieldPermissionsTable({ project }: { readonly project?: string }) {
  const levels = useFieldPermissions(project);
  return (
    <LevelTableView
      levels={levels}
      subjectName={(field) => t(`issue-field.${field}`)}
      levelName={(level) => t(`field-level.${level}`)}
      subjectHeading={t("access.field")}
      tableName={t("field-permissions.table-name")}
    />
  );
}

/** What a person with the chosen role sees of people, under the table. */
function preview(table: LevelTable<PeopleKindKey, PeopleLevelCode>, role: string) {
  return <PeoplePreview table={table} role={role} />;
}

/** *Who sees people* of a place: the whole organization, or a project (an ID). */
export function PeopleVisibilityTable({ project }: { readonly project?: string }) {
  const levels = usePeopleVisibility(project);
  return (
    <LevelTableView
      levels={levels}
      subjectName={(kind) => t(`people-kind.${kind}`)}
      levelName={(level) => t(`people-level.${level}`)}
      subjectHeading={t("access.kind")}
      tableName={t("people-visibility.table-name")}
      preferredRole={t("starting-role.guest")}
      previewLabel={t("people-visibility.preview-role")}
      after={preview}
    />
  );
}
