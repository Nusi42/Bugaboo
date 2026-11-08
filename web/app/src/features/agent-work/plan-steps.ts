/** The mark that begins a numbered step: `1.` or `1)`. */
const STEP_MARK = /^\s*\d+[.)]\s+/u;

/** Where two steps that stand on one line meet: `1. Read. 2. Fix.` */
const STEPS_ON_ONE_LINE = /\s+(?=\d+[.)]\s)/u;

/**
 * The steps of a plan the agent reported as text, one for each line, or for each number when the
 * agent wrote them on one line. A plan with no steps is one step.
 */
export function planSteps(plan: string): readonly string[] {
  const lines = plan
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter((line) => line !== "");
  const pieces = lines.flatMap((line) =>
    STEP_MARK.test(line) ? line.split(STEPS_ON_ONE_LINE) : [line],
  );
  return pieces.map((piece) => piece.replace(STEP_MARK, "").trim()).filter((piece) => piece !== "");
}
