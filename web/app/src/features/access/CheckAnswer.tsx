import { isMessageKey, locale, t } from "@bugaboo/messages";
import { Button, buttonLook, Card, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { personName } from "../../shared/person.ts";
import { levelSentences, reasonSentences } from "./check-sentences.ts";
import { fieldNameOf } from "./field-names.ts";
import { groupLevels, type LevelEntry } from "./group-levels.ts";
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
    <li {...partOf(part.accessAnswer)}>
      <p {...partOf(part.accessSubheading)}>{t(`permission.${permission.permission}`)}</p>
      <p {...partOf(part.accessReasons)}>
        <strong
          {...partOf(part.accessAnswerVerdict, {
            variants: [permission.allowed ? "yes" : "no"],
          })}
        >
          {permission.allowed ? t("check-access.yes") : t("check-access.no")}
        </strong>
        {sentences.join(" ")}
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
        <section key={topic} {...partOf(part.stack)}>
          <h3 {...partOf(part.accessTopic)}>{t(`permission-topic.${topic}`)}</h3>
          <ul {...partOf(part.list)}>
            {permissions.map((permission) => (
              <PermissionLine key={permission.permission} answer={answer} permission={permission} />
            ))}
          </ul>
        </section>
      ))}
    </Card>
  );
}

interface LevelSectionProps {
  readonly answer: CheckAnswerBody;
  readonly heading: string;
  /** The fields or the kinds, each with what it is called. */
  readonly entries: readonly (LevelEntry & {
    readonly says: FieldAnswer["roles"] | KindAnswer["roles"];
    readonly cappedByKey: boolean;
  })[];
  readonly levelName: (level: string) => string;
  /** The words for the group of everything, the group of the rest, and the two ways to look. */
  readonly words: {
    readonly all: string;
    readonly allOther: string;
    readonly every: string;
  };
}

/** How the names of a group are told: a list in words. */
function listOf(names: readonly string[]): string {
  return new Intl.ListFormat(locale, { type: "conjunction" }).format(names);
}

/**
 * The fields (or kinds) with their level and the reasons. Fields that have the same level for the
 * same reason are told together, so each reason is read once; every one can be shown on its own.
 */
function LevelSection({ answer, heading, entries, levelName, words }: LevelSectionProps) {
  const [every, setEvery] = useState(false);
  const told: readonly LevelEntry[] = entries.map((entry) => ({
    name: entry.name,
    level: entry.level,
    sentences: levelSentences(answer.person, levelName, entry.says, entry.cappedByKey),
  }));
  const groups = groupLevels(told);
  return (
    <Card heading={heading}>
      <div {...partOf(part.row)}>
        <Button
          look={buttonLook.quiet}
          onPress={() => {
            setEvery(!every);
          }}
        >
          {every ? t("check-access.group-the-same") : words.every}
        </Button>
      </div>
      <ul {...partOf(part.list)}>
        {every
          ? told.map((entry) => (
              <li key={entry.name} {...partOf(part.accessAnswer)}>
                <div {...partOf(part.accessAnswerHead)}>
                  <p {...partOf(part.accessSubheading)}>{entry.name}</p>
                  <span {...partOf(part.strong)}>{levelName(entry.level)}</span>
                </div>
                <p {...partOf(part.accessReasons)}>{entry.sentences.join(" ")}</p>
              </li>
            ))
          : groups.map((group, at) => (
              <li key={group.names.join()} {...partOf(part.accessAnswer)}>
                <div {...partOf(part.accessAnswerHead)}>
                  <p {...partOf(part.accessSubheading)}>
                    {groups.length === 1
                      ? words.all
                      : at === 0
                        ? words.allOther
                        : listOf(group.names)}
                  </p>
                  <span {...partOf(part.strong)}>{levelName(group.level)}</span>
                </div>
                <p {...partOf(part.accessReasons)}>{group.sentences.join(" ")}</p>
              </li>
            ))}
      </ul>
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
      <LevelSection
        answer={answer}
        heading={t("check-access.fields-heading")}
        entries={answer.fields.map((field) => ({
          name: fieldNameOf({ code: field.subject, name: field.name }),
          level: field.level,
          sentences: [],
          says: field.roles,
          cappedByKey: field.capped_by_key,
        }))}
        levelName={fieldLevelName}
        words={{
          all: t("check-access.all-fields"),
          allOther: t("check-access.all-other-fields"),
          every: t("check-access.show-every-field"),
        }}
      />
      <LevelSection
        answer={answer}
        heading={t("check-access.people-heading")}
        entries={answer.kinds.map((kind) => ({
          name: t(`people-kind.${kind.subject}`),
          level: kind.level,
          sentences: [],
          says: kind.roles,
          cappedByKey: kind.capped_by_key,
        }))}
        levelName={peopleLevelName}
        words={{
          all: t("check-access.all-kinds"),
          allOther: t("check-access.all-other-kinds"),
          every: t("check-access.show-every-kind"),
        }}
      />
    </>
  );
}
