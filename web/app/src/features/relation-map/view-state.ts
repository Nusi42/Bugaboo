/** How the relation map is shown: as the picture, or as the list of its chains (UX §4.15). */
export type MapForm = "picture" | "list";

/** The zooms a person can choose, as parts of the size of the text; *Fit* picks the largest that fits. */
export const zoomSteps: readonly number[] = [0.25, 0.5, 0.75, 1, 1.25, 1.5];

/** The size of the text: what a map starts at. */
export const fullSize = 1;

/** The largest zoom at which the whole picture fits, never more than the size of the text and never less than the smallest. */
export function fittingZoom(fits: number): number {
  const below = zoomSteps.filter((step) => step <= Math.min(fits, fullSize));
  return below.at(-1) ?? zoomSteps[0] ?? fullSize;
}
