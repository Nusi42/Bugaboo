import type { Schema } from "@bugaboo/api-client";
import type { MessageKey } from "@bugaboo/messages";

/** What a key may do, as the API names it. */
export type Choice = Schema<"KeyChoice">;

/** The three plain choices of *What it may do*; single permissions are the fourth way. */
export type PlainChoice = Exclude<Choice, "permissions">;

/** The choices in the order they are offered. */
export const plainChoices: readonly PlainChoice[] = ["see-only", "see-and-change", "everything"];

const names: Readonly<Record<PlainChoice, MessageKey>> = {
  "see-only": "access-keys.choice.see-only",
  "see-and-change": "access-keys.choice.see-and-change",
  everything: "access-keys.choice.everything",
};

const helps: Readonly<Record<Choice, MessageKey>> = {
  "see-only": "access-keys.help.see-only",
  "see-and-change": "access-keys.help.see-and-change",
  everything: "access-keys.help.everything",
  permissions: "access-keys.help.permissions",
};

/** The name of a plain choice, as the glossary words it. */
export function choiceName(choice: PlainChoice): MessageKey {
  return names[choice];
}

/** The sentence that says what a way of choosing means. */
export function choiceHelp(choice: Choice): MessageKey {
  return helps[choice];
}

/** The sentence of a permission, as the catalogue says it (`permission.<key>`). */
export function permissionSentence(permission: Schema<"Permission">): MessageKey {
  return `permission.${permission}`;
}

/** The heading of a topic of the permissions (`permission-topic.<topic>`). */
export function topicHeading(topic: Schema<"Topic">): MessageKey {
  return `permission-topic.${topic}`;
}

/** The permissions of one topic. */
export interface TopicGroup {
  readonly topic: Schema<"Topic">;
  readonly permissions: readonly Schema<"Permission">[];
}

/**
 * The permissions to choose from, grouped by topic in the order the catalogue lists them (the
 * permissions of a topic stand together there).
 */
export function topicGroups(
  choices: readonly Schema<"PermissionChoiceBody">[],
): readonly TopicGroup[] {
  const groups: { topic: Schema<"Topic">; permissions: Schema<"Permission">[] }[] = [];
  for (const choice of choices) {
    const last = groups.at(-1);
    if (last?.topic === choice.topic) {
      last.permissions.push(choice.key);
    } else {
      groups.push({ topic: choice.topic, permissions: [choice.key] });
    }
  }
  return groups;
}
