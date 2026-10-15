import type { Schema } from "@bugaboo/api-client";
import { isMessageKey, t } from "@bugaboo/messages";

import { fieldNameOf } from "../../shared/field-names.ts";
import { listText } from "../../shared/list-text.ts";
import { personName } from "../../shared/person.ts";

import type { Preview } from "./use-projects.ts";

type Change = Preview["people"][number]["projects"][number];
type Word = Schema<"WordBody">;

/** The text of a message the server named, or the word itself for a message this app lacks. */
function say(key: string, fallback: string): string {
  return isMessageKey(key) ? t(key) : fallback;
}

/** What a word of the vocabulary is called: a field, a kind of people information, a level. */
function wordOf(word: Word): string {
  return say(word.message_key, word.code);
}

/** The names of roles in a sentence; *no role* when there are none. */
function rolesOf(roles: readonly string[]): string {
  return roles.length === 0 ? t("projects.no-role") : listText(roles);
}

/** The sentences of what one permission list says: what a person can now do or no more. */
function permissionsOf(keys: readonly string[]): string {
  return keys
    .map((key) => {
      const message = `permission.${key}`;
      return say(message, key);
    })
    .join("; ");
}

/** A line of the preview about access: gaining it, losing it, changing role. */
function accessLine(who: string, change: Change): string | undefined {
  const project = change.project.name;
  const { now, after } = change;
  if (!now.sees && after.sees) {
    return t("projects.line-gains", { person: who, project, roles: rolesOf(after.roles) });
  }
  if (now.sees && !after.sees) {
    return t("projects.line-loses", { person: who, project });
  }
  if (now.roles.join() !== after.roles.join()) {
    return t("projects.line-roles", {
      person: who,
      project,
      before: rolesOf(now.roles),
      after: rolesOf(after.roles),
    });
  }
  return undefined;
}

/** The lines about a No access that starts or stops holding the person back. */
function heldBackLine(who: string, change: Change): string | undefined {
  const project = change.project.name;
  if (change.now.held_back && !change.after.held_back) {
    return t("projects.line-held-back-ends", { person: who, project });
  }
  if (!change.now.held_back && change.after.held_back) {
    return t("projects.line-held-back-starts", { person: who, project });
  }
  return undefined;
}

/** What a person can do now or no more, for a person who sees the project before and after. */
function abilityLines(who: string, change: Change): string[] {
  if (!change.now.sees || !change.after.sees) {
    return [];
  }
  const project = change.project.name;
  const lines: string[] = [];
  if (change.gains.length > 0) {
    lines.push(
      t("projects.line-can-now", { person: who, project, things: permissionsOf(change.gains) }),
    );
  }
  if (change.loses.length > 0) {
    lines.push(
      t("projects.line-can-no-more", { person: who, project, things: permissionsOf(change.loses) }),
    );
  }
  return lines;
}

/** The lines about levels of fields and of people information. */
function levelLines(who: string, change: Change): string[] {
  const project = change.project.name;
  return [
    ...change.fields.map((level) =>
      t("projects.line-field", {
        person: who,
        project,
        field: fieldNameOf(level.field),
        before: wordOf(level.before),
        after: wordOf(level.after),
      }),
    ),
    ...change.people.map((level) =>
      t("projects.line-people", {
        person: who,
        project,
        kind: wordOf(level.kind),
        before: wordOf(level.before),
        after: wordOf(level.after),
      }),
    ),
  ];
}

/** Every sentence a change makes, in plain words. */
function sentencesOf(who: string, change: Change): string[] {
  return [
    accessLine(who, change),
    heldBackLine(who, change),
    ...abilityLines(who, change),
    ...levelLines(who, change),
  ].filter((line): line is string => line !== undefined);
}

/** A sentence of the preview, with a key that names whose change in which project it is. */
export interface Sentence {
  readonly key: string;
  readonly text: string;
}

/** The sentences about one person, in all the projects the person sees. */
export interface PersonSentences {
  readonly key: string;
  /** Who the sentences are about. */
  readonly person: string;
  readonly lines: readonly Sentence[];
}

/** The sentences of the preview grouped by person, for the long preview; a person with none is left out. */
export function personSentencesOfPreview(preview: Preview): readonly PersonSentences[] {
  return preview.people
    .map((entry, person): PersonSentences => ({
      key: String(person),
      person: personName(entry.person),
      lines: entry.projects.flatMap((change) =>
        sentencesOf(personName(entry.person), change).map((text, line) => ({
          key: `${change.project.id}:${String(line)}`,
          text,
        })),
      ),
    }))
    .filter((group) => group.lines.length > 0);
}

/** The sentences about people who could take away a No access that applies to them, with keys. */
export function keyedLiftSentencesOfPreview(preview: Preview): readonly Sentence[] {
  return preview.lifts.map((lift, person) => ({
    key: `${String(person)}:${lift.project.id}`,
    text: t("projects.line-lift", { person: personName(lift.person), project: lift.project.name }),
  }));
}
