import { priorityWords, statusWords } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

/** The statuses to pick from, each with its words. */
export const statusChoices = statusWords.map((word) => ({
  value: word,
  label: t(`status.${word}`),
}));

/** The priorities to pick from, each with its words: the issue page and the new-issue form pick from the same list. */
export const priorityChoices = priorityWords.map((word) => ({
  value: word,
  label: t(`priority.${word}`),
}));
