import { t } from "@bugaboo/messages";

import { listWord } from "../../shared/list-columns.ts";

/** What the date field of a calendar is called in the sentences of the screen. */
export interface DateWords {
  /** The name of the field at the start of a sentence or as the name of a box: "Due date". */
  readonly label: string;
  /** In a sentence, after the ID of an issue: "due date". */
  readonly noun: string;
  /** The name of the button and the window that change it: "Change due date …". */
  readonly change: string;
}

/**
 * The words of the date field a view stands on (`null` is *Due date*): the two built-in dates have words of
 * their own, an own date field is called by the name the person gave it.
 */
export function dateWordsOf(
  word: string | null,
  names: Readonly<Record<string, string>>,
): DateWords {
  if (word === null || word === listWord.dueDate) {
    return {
      label: t("issue-field.due-date"),
      noun: t("calendar.due-date"),
      change: t("calendar.change-due-date"),
    };
  }
  if (word === listWord.startDate) {
    return {
      label: t("issue-field.start-date"),
      noun: t("calendar.start-date"),
      change: t("calendar.change-start-date"),
    };
  }
  const name = names[word] ?? word;
  return { label: name, noun: name, change: t("calendar.change-own-date", { name }) };
}
