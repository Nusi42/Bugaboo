import { type Page } from "@playwright/test";

import { goodWindowInFourThemes } from "./demo.ts";
import { inFourThemesAsWindow } from "./four-themes.ts";
import { sampleNamed, type SamplePerson } from "./sample.ts";

// What the steps of the exit demo of M7 share (`m7-exit-demo.spec.ts`): the pictures and the people.

/** How long a step of the demo may take: a step signs in, works and takes its pictures in four themes. */
export const STEP_TIMEOUT_MS = 300_000;

/** The pictures of the demo, in the folder of the screenshots. */
const DEMO_M7 = "M7/exit-demo";

/** The screen is good in the four themes and is in the pictures of this demo. */
export function good(page: Page, step: string): Promise<void> {
  return goodWindowInFourThemes(page, step, DEMO_M7);
}

/** Ada Brandt, the first Administrator of the sample. */
export function adaOfTheSample(): SamplePerson {
  return sampleNamed("Ada Brandt");
}

/** Ana Moreno, a Contributor in Website through the group External. */
export function anaOfTheSample(): SamplePerson {
  return sampleNamed("Ana Moreno");
}

/** Ben Hale, the Team member whose two agents the sample holds. */
export function benOfTheSample(): SamplePerson {
  return sampleNamed("Ben Hale");
}

/**
 * The screen is in the pictures of this demo in the four themes, without the check of the names of
 * controls: the issue page has label chips whose visible words are not in their names, and the
 * assistant panel its *Leave out* buttons (M7-F126).
 */
export function shot(page: Page, step: string): Promise<void> {
  return inFourThemesAsWindow(page, step, DEMO_M7);
}
