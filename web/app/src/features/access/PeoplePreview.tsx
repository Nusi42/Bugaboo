import { t } from "@bugaboo/messages";

import { Card } from "@bugaboo/ui";

import styles from "./Access.module.css";
import type { LevelTable, PeopleKindKey, PeopleLevelCode } from "./levels.ts";

/** What a person looks like to somebody who sees people at this level: *Ana*, *a Team member*, *Someone*. */
function who(level: PeopleLevelCode, startOfSentence: boolean): string {
  if (level === "names") {
    return t("people-visibility.who-names");
  }
  if (level === "role-only") {
    const role = t("starting-role.team-member");
    return startOfSentence
      ? t("projects.person-role", { role })
      : t("access.person-role-inside", { role });
  }
  return t("projects.person-someone");
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
      <ul className={styles.previewLines}>
        <li className={styles.previewLine}>
          {t("people-visibility.example-assigned", { who: who(assigned, false) })}
        </li>
        <li className={styles.previewLine}>
          {t("people-visibility.example-comment", { who: who(authors, false) })}
        </li>
        <li className={styles.previewLine}>
          {t("people-visibility.example-editing", { who: who(editing, true) })}
        </li>
      </ul>
    </Card>
  );
}
