import { statusGroupWords } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

/** The status groups, by the words people read, in the order the server lists them. */
export const groupChoices = statusGroupWords.map((word) => ({
  value: word,
  label: t(`status-group.${word}`),
}));
