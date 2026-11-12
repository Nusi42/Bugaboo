import { dateEffectWords } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

type DateEffectWord = (typeof dateEffectWords)[number];

/** The words of each date effect: an effect that joins the list (V1) is a type error until it has them. */
const effectWords: Readonly<Record<DateEffectWord, string>> = {
  none: t("relation-kinds.date-effect.none"),
  "must-finish-first": t("relation-kinds.date-effect.must-finish-first"),
};

/**
 * The date effects a kind of relation can have (ADR-0043, decision 4), in the order the server lists
 * them: *None*, then *Must finish first*.
 */
export const dateEffectChoices = dateEffectWords.map((word) => ({
  value: word,
  label: effectWords[word],
}));
