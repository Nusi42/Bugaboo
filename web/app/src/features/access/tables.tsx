import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { fieldNameOf } from "../../shared/field-names.ts";
import { IssueTypeChooser } from "../issue-types/index.ts";
import { LevelTableView } from "./LevelTableView.tsx";
import type { LevelTable, PeopleKindKey, PeopleLevelCode } from "./levels.ts";
import { PeoplePreview } from "./PeoplePreview.tsx";
import { useFieldPermissions, usePeopleVisibility } from "./use-levels.ts";

/** *More options* of the field permissions: the issue type whose levels the table shows (ADR-0027). */
function TypeOptions({
  type,
  onChange,
}: {
  readonly type: string | undefined;
  readonly onChange: (issueType: string | undefined) => void;
}) {
  return (
    <div {...partOf(part.stack)}>
      <IssueTypeChooser
        label={t("field-permissions.type-chooser")}
        type={type}
        onChange={onChange}
      />
      <p {...partOf(part.muted)}>
        {type === undefined
          ? t("field-permissions.type-help")
          : t("field-permissions.type-help-chosen")}
      </p>
    </div>
  );
}

/** The field permissions of a place: the whole organization, or a project (an ID). */
export function FieldPermissionsTable({ project }: { readonly project?: string }) {
  const [more, setMore] = useState(false);
  const [issueType, setIssueType] = useState<string>();
  const levels = useFieldPermissions(project, issueType);
  return (
    <div {...partOf(part.stack)}>
      <div {...partOf(part.row)}>
        <Button
          look={buttonLook.quiet}
          onPress={() => {
            if (more) {
              setIssueType(undefined);
            }
            setMore(!more);
          }}
        >
          {more ? t("field-permissions.fewer-options") : t("field-permissions.more-options")}
        </Button>
      </div>
      {more ? <TypeOptions type={issueType} onChange={setIssueType} /> : null}
      <LevelTableView
        levels={levels}
        subjectName={(field, name) => fieldNameOf({ code: field, name })}
        levelName={(level) => t(`field-level.${level}`)}
        subjectHeading={t("access.field")}
        tableName={t("field-permissions.table-name")}
      />
    </div>
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
