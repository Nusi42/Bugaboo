import { t, type MessageKey } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { fieldNameOf } from "../../shared/field-names.ts";
import { roleName, someoneName } from "../../shared/person.ts";
import { IssueTypeChooser } from "../issue-types/index.ts";
import { LevelTableView } from "./LevelTableView.tsx";
import type { FieldKeyCode, FieldLevelCode, PeopleLevelCode } from "./levels.ts";
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

/** What a field is called: an own field by its name, a field of the catalogue by its message. */
function fieldName(field: FieldKeyCode, name: string | null | undefined): string {
  return fieldNameOf({ code: field, name });
}

/** The sentence of each level of a field, which names the fields the role has at that level. */
const fieldSentenceOf = {
  "can-change": "field-permissions.preview-can-change",
  "can-see": "field-permissions.preview-can-see",
  hidden: "field-permissions.preview-hidden",
} as const satisfies Readonly<Record<FieldLevelCode, MessageKey>>;

/** What a role sees of the fields at a level, in a sentence of the preview. */
function fieldSentence(level: FieldLevelCode, fields: string): string {
  return t(fieldSentenceOf[level], { fields });
}

/** What a role sees of people in the kinds at a level, in a sentence of the preview (the words of the glossary). */
function peopleSentence(level: PeopleLevelCode, kinds: string): string {
  if (level === "names") {
    return t("people-visibility.preview-names", { kinds });
  }
  if (level === "role-only") {
    return t("people-visibility.preview-role-only", {
      kinds,
      example: roleName("inside", t("starting-role.team-member")),
    });
  }
  return t("people-visibility.preview-hidden", { kinds, someone: someoneName("inside") });
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
        subjectName={fieldName}
        levelName={(level) => t(`field-level.${level}`)}
        subjectHeading={t("access.field")}
        tableName={t("field-permissions.table-name")}
        sentence={fieldSentence}
      />
    </div>
  );
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
      sentence={peopleSentence}
    />
  );
}
