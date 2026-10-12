import { t } from "@bugaboo/messages";
import { Button, buttonLook, SelectField, partOf, part } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { fieldNameOf } from "./field-names.ts";
import { LevelTableView } from "./LevelTableView.tsx";
import type { LevelTable, PeopleKindKey, PeopleLevelCode } from "./levels.ts";
import { PeoplePreview } from "./PeoplePreview.tsx";
import { useFieldPermissions, usePeopleVisibility } from "./use-levels.ts";

/** The value of the choice *All issue types* in the list of issue types. */
const allTypes = "";

/** *More options* of the field permissions: the issue type whose levels the table shows (ADR-0027). */
function IssueTypeChooser({
  value,
  onChange,
}: {
  readonly value: string;
  readonly onChange: (issueType: string) => void;
}) {
  const types = useQuery(api.query.queryOptions("get", "/api/v1/issue-types"));
  return (
    <div {...partOf(part.stack)}>
      <SelectField
        label={t("field-permissions.type-chooser")}
        choices={[
          { value: allTypes, label: t("field-permissions.all-types") },
          ...(types.data?.items ?? []).flatMap((each) =>
            each.id === undefined ||
            each.id === null ||
            each.name === undefined ||
            each.name === null
              ? []
              : [{ value: each.id, label: each.name }],
          ),
        ]}
        value={value}
        onChange={onChange}
      />
      <p {...partOf(part.muted)}>
        {value === allTypes
          ? t("field-permissions.type-help")
          : t("field-permissions.type-help-chosen")}
      </p>
    </div>
  );
}

/** The field permissions of a place: the whole organization, or a project (an ID). */
export function FieldPermissionsTable({ project }: { readonly project?: string }) {
  const [more, setMore] = useState(false);
  const [issueType, setIssueType] = useState(allTypes);
  const levels = useFieldPermissions(project, issueType === allTypes ? undefined : issueType);
  return (
    <div {...partOf(part.stack)}>
      <div {...partOf(part.row)}>
        <Button
          look={buttonLook.quiet}
          onPress={() => {
            if (more) {
              setIssueType(allTypes);
            }
            setMore(!more);
          }}
        >
          {more ? t("field-permissions.fewer-options") : t("field-permissions.more-options")}
        </Button>
      </div>
      {more ? <IssueTypeChooser value={issueType} onChange={setIssueType} /> : null}
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
