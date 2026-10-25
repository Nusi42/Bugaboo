/**
 * Does the steps one after the other. A browser does one thing at a time, so the steps of a test
 * cannot run side by side.
 */
export async function inOrder(steps: readonly (() => Promise<unknown>)[]): Promise<void> {
  const [first, ...rest] = steps;
  if (first !== undefined) {
    await first();
    await inOrder(rest);
  }
}

/** A step that has nothing to do (a layout where there is nothing to look at, a preparation that needs none). */
export const nothing = (): Promise<void> => Promise.resolve();
