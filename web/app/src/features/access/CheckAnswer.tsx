import { isMessageKey, t } from "@bugaboo/messages";
import { Card } from "@bugaboo/ui";

import { personName } from "../../shared/person.ts";
import styles from "./Access.module.css";
import { levelSentences, reasonSentences } from "./check-sentences.ts";
import { projectText } from "./origin-text.ts";
import type { CheckAnswerBody } from "./use-check.ts";

type PermissionAnswer = CheckAnswerBody["permissions"][number];
type FieldAnswer = CheckAnswerBody["fields"][number];
type KindAnswer = CheckAnswerBody["kinds"][number];

/** The name of a level of a field; the word itself for one this version of the app does not know. */
function fieldLevelName(level: string): string {
  const key = `field-level.${level}`;
  return isMessageKey(key) ? t(key) : level;
}

/** The name of a level of people information; the word itself for one this version does not know. */
function peopleLevelName(level: string): string {
  const key = `people-level.${level}`;
  return isMessageKey(key) ? t(key) : level;
}

/** The permissions grouped by what they are about, in the order the catalogue lists them. */
function byTopic(
  permissions: readonly PermissionAnswer[],
): readonly (readonly [PermissionAnswer["topic"], readonly PermissionAnswer[]])[] {
  const groups = new Map<PermissionAnswer["topic"], PermissionAnswer[]>();
  for (const permission of permissions) {
    groups.set(permission.topic, [...(groups.get(permission.topic) ?? []), permission]);
  }
  return [...groups.entries()];
}

interface PermissionLineProps {
  readonly answer: CheckAnswerBody;
  readonly permission: PermissionAnswer;
}

/** One permission: what it is, *Yes.* or *No.*, and the reasons in sentences. */
function PermissionLine({ answer, permission }: PermissionLineProps) {
  const sentences = permission.reasons.flatMap((reason) =>
    reasonSentences(answer.person, answer.project, reason),
  );
  return (
    <li className={styles.answer}>
      <p className={styles.answerName}>{t(`permission.${permission.permission}`)}</p>
      <p className={styles.reasons}>
        <strong className={permission.allowed ? styles.verdictYes : styles.verdictNo}>
          {permission.allowed ? t("check-access.yes") : t("check-access.no")}
        </strong>
        {sentences.join(" ")}
      </p>
    </li>
  );
}

interface LevelLineProps {
  readonly answer: CheckAnswerBody;
  readonly subject: string;
  readonly level: string;
  readonly levelName: (level: string) => string;
  readonly says: FieldAnswer["roles"] | KindAnswer["roles"];
  readonly cappedByKey: boolean;
}

/** One field or kind: its name and level, and why in sentences. */
function LevelLine({ answer, subject, level, levelName, says, cappedByKey }: LevelLineProps) {
  return (
    <li className={styles.answer}>
      <div className={styles.answerHead}>
        <p className={styles.answerName}>{subject}</p>
        <span className={styles.verdict}>{levelName(level)}</span>
      </div>
      <p className={styles.reasons}>
        {levelSentences(answer.person, subject, levelName, says, cappedByKey).join(" ")}
      </p>
    </li>
  );
}

/** What the person can do here, by topic, under the heading that says who and where. */
function Permissions({
  answer,
  heading,
}: {
  readonly answer: CheckAnswerBody;
  readonly heading: string;
}) {
  return (
    <Card heading={heading}>
      {byTopic(answer.permissions).map(([topic, permissions]) => (
        <section key={topic} className={styles.stack}>
          <h3 className={styles.topic}>{t(`permission-topic.${topic}`)}</h3>
          <ul className={styles.answers}>
            {permissions.map((permission) => (
              <PermissionLine key={permission.permission} answer={answer} permission={permission} />
            ))}
          </ul>
        </section>
      ))}
    </Card>
  );
}

/** The answer of *Check access*: what the person can do, every field, and how they see people. */
export function CheckAnswer({ answer }: { readonly answer: CheckAnswerBody }) {
  const place =
    answer.project === undefined || answer.project === null
      ? t("check-access.in-organization")
      : projectText(answer.project);
  return (
    <>
      <Permissions
        answer={answer}
        heading={t("check-access.result-heading", { person: personName(answer.person), place })}
      />
      <Card heading={t("check-access.fields-heading")}>
        <ul className={styles.answers}>
          {answer.fields.map((field) => (
            <LevelLine
              key={field.subject}
              answer={answer}
              subject={t(`issue-field.${field.subject}`)}
              level={field.level}
              levelName={fieldLevelName}
              says={field.roles}
              cappedByKey={field.capped_by_key}
            />
          ))}
        </ul>
      </Card>
      <Card heading={t("check-access.people-heading")}>
        <ul className={styles.answers}>
          {answer.kinds.map((kind) => (
            <LevelLine
              key={kind.subject}
              answer={answer}
              subject={t(`people-kind.${kind.subject}`)}
              level={kind.level}
              levelName={peopleLevelName}
              says={kind.roles}
              cappedByKey={kind.capped_by_key}
            />
          ))}
        </ul>
      </Card>
    </>
  );
}
