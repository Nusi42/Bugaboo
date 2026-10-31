import { t, type MessageKey } from "@bugaboo/messages";
import { Card, partOf, part } from "@bugaboo/ui";

import { listText } from "../../shared/list-text.ts";
import type { FieldKeyCode, FieldLevelCode, LevelTable } from "./levels.ts";

/** The sentence of each level, which names the fields the role has at that level. */
const sentenceOf = {
  "can-change": "field-permissions.preview-can-change",
  "can-see": "field-permissions.preview-can-see",
  hidden: "field-permissions.preview-hidden",
} as const satisfies Readonly<Record<FieldLevelCode, MessageKey>>;

/** One line of the preview: a level and the sentence that names its fields. */
interface PreviewLine {
  readonly level: FieldLevelCode;
  readonly text: string;
}

export interface FieldsPreviewProps {
  readonly table: LevelTable<FieldKeyCode, FieldLevelCode>;
  /** The ID of the role whose view is shown. */
  readonly role: string;
  /** What a field is called. */
  readonly fieldName: (field: FieldKeyCode, name: string | null | undefined) => string;
}

/**
 * What a person with the role sees of the fields of an issue, level by level from the highest: the fields they can
 * change, the ones they can only see, and the hidden ones. So an administrator reads the effect of the column before
 * anybody else does, as *Who sees people* shows its sentences.
 */
export function FieldsPreview({ table, role, fieldName }: FieldsPreviewProps) {
  const name = table.roles.find((known) => known.id === role)?.name ?? "";
  const lines = table.levels.toReversed().flatMap((level): readonly PreviewLine[] => {
    const fields = table.rows
      .filter((row) => row.cells.some((cell) => cell.role === role && cell.level === level))
      .map((row) => fieldName(row.subject, row.name));
    return fields.length === 0
      ? []
      : [{ level, text: t(sentenceOf[level], { fields: listText(fields) }) }];
  });
  return (
    <Card heading={t("access.preview-heading", { role: name })}>
      <ul {...partOf(part.list, { variants: ["tight"] })}>
        {lines.map((line) => (
          <li key={line.level} {...partOf(part.accessPreviewLine)}>
            {line.text}
          </li>
        ))}
      </ul>
    </Card>
  );
}
