import { type Page } from "@playwright/test";

import { goodWindowInFourThemes } from "./demo.ts";
import { sampleNamed, type SamplePerson } from "./sample.ts";

// What the steps of the exit demo of M6 share (`m6-exit-demo.spec.ts`): the pictures, Ana and the days.

/** How long a step of the demo may take: a step signs in, taps and takes its pictures in four themes. */
export const STEP_TIMEOUT_MS = 240_000;

/** The pictures of the demo, in the folder of the screenshots. */
const DEMO_M6 = "M6/demo";
/** The screen is good in the four themes and is in the pictures of this demo. */
export function good(page: Page, step: string): Promise<void> {
  return goodWindowInFourThemes(page, step, DEMO_M6);
}

/** Ana Moreno, a Contributor in Website through the group External. */
export function ana(): SamplePerson {
  return sampleNamed("Ana Moreno");
}
