import { t } from "@bugaboo/messages";

import type { ChainStep } from "./layout.ts";

/**
 * The words of a chain (UX §4.15): "WEB-142 blocks WEB-147, which is blocked by WEB-148". Each
 * relation is told in the words its first issue reads, as the server sends them; only the joints
 * of the sentence come from the catalog.
 */

/** The sentence up to the last issue: "WEB-142 blocks WEB-147, which is blocked by". */
export function leadOf(steps: readonly ChainStep[]): string {
  let lead = "";
  for (const [at, step] of steps.entries()) {
    lead =
      at === 0
        ? t("relation-map.lead-first", { key: step.from.readable_id, word: step.word })
        : t("relation-map.lead-next", { lead, key: step.from.readable_id, word: step.word });
  }
  return lead;
}

/** The whole sentence of a chain, ending with its last issue; none for a chain without a step. */
export function sentenceOf(steps: readonly ChainStep[]): string | undefined {
  const last = steps.at(-1);
  return last === undefined
    ? undefined
    : t("relation-map.sentence", { lead: leadOf(steps), key: last.to.readable_id });
}

/** One relation as a sentence of its own: "WEB-147 is blocked by WEB-148". */
export function stepSentence(step: ChainStep): string {
  return sentenceOf([step]) ?? step.to.readable_id;
}

/** What a loop says: its relations round it, and that nothing in it can start. */
export function loopSentence(steps: readonly ChainStep[]): string | undefined {
  const sentence = sentenceOf(steps);
  return sentence === undefined ? undefined : t("relation-map.loop", { sentence });
}
