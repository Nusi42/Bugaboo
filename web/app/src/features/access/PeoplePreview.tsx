import { t } from "@bugaboo/messages";
import { Card, partOf, part } from "@bugaboo/ui";

import { roleName, someoneName, type Position } from "../../shared/person.ts";
import type { LevelTable, PeopleKindKey, PeopleLevelCode } from "./levels.ts";

/** What a person looks like to somebody who sees people at this level: *Ana*, *a Team member*, *Someone*. */
function who(level: PeopleLevelCode, position: Position): string {
  if (level === "names") {
    return t("people-visibility.who-names");
  }
  if (level === "role-only") {
    return roleName(position, t("starting-role.team-member"));
  }
  return someoneName(position);
}

/** The level a role has for a kind of people information in the table. */
function levelOf(
  table: LevelTable<PeopleKindKey, PeopleLevelCode>,
  role: string,
  kind: PeopleKindKey,
): PeopleLevelCode {
  const cell = table.rows.find((row) => row.subject === kind)?.cells.find((c) => c.role === role);
  return cell?.level ?? "hidden";
}

export interface PeoplePreviewProps {
  readonly table: LevelTable<PeopleKindKey, PeopleLevelCode>;
  /** The ID of the role whose view is shown. */
  readonly role: string;
}

/**
 * What a person with the role sees of people, in sentences: who is assigned, who wrote a comment,
 * who is editing. So an administrator reads the effect of the table before anybody else does.
 */
export function PeoplePreview({ table, role }: PeoplePreviewProps) {
  const name = table.roles.find((known) => known.id === role)?.name ?? "";
  const assigned = levelOf(table, role, "assigned-to");
  const authors = levelOf(table, role, "authors");
  const editing = levelOf(table, role, "editing");
  return (
    <Card heading={t("people-visibility.preview-heading", { role: name })}>
      <ul {...partOf(part.list, { variants: ["tight"] })}>
        <li {...partOf(part.accessPreviewLine)}>
          {t("people-visibility.example-assigned", { who: who(assigned, "inside") })}
        </li>
        <li {...partOf(part.accessPreviewLine)}>
          {t("people-visibility.example-comment", { who: who(authors, "inside") })}
        </li>
        <li {...partOf(part.accessPreviewLine)}>
          {t("people-visibility.example-editing", { who: who(editing, "start") })}
        </li>
      </ul>
    </Card>
  );
}
