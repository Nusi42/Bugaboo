import { priorityWords } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

/** One word to pick, with the words people read for it. */
export interface Choice<Word extends string> {
  readonly value: Word;
  readonly label: string;
}

/** The priorities to pick from, each with its words: the issue page and the new-issue form pick from the same list. */
export const priorityChoices: readonly Choice<(typeof priorityWords)[number]>[] = priorityWords.map(
  (word) => ({
    value: word,
    label: t(`priority.${word}`),
  }),
);
