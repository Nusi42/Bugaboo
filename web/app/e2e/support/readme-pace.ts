// How fast the recordings of the front page play. They are watched by people who have never seen
// the app, so every state they should notice stays long enough to be read, the pointer moves as a
// hand does, and each recording ends on a pause before it starts again. Holding a picture costs a
// GIF almost nothing: it is one frame with a longer time.

/** The timing of the recordings (`recording.ts`, `readme-recordings.ts`), in milliseconds. */
export const PACE = {
  /** One picture of a movement of the pointer. */
  stepMs: 70,
  /** How many pictures the pointer takes from one place to the next: about a second. */
  moveSteps: 16,
  /** The pointer rests on what it presses before the press, so the eye finds it. */
  pointedMs: 500,
  /** One typed letter. */
  letterMs: 200,
  /** The screen before anything happens: the list. */
  startMs: 2000,
  /** A moment to see what just opened (quick search, the window for a new issue). */
  lookMs: 1500,
  /** What a person should read: the results that were found, the title that was typed. */
  resultMs: 2500,
  /** The last state (the issue that opened), and the pause before the recording starts again. */
  endMs: 4000,
} as const;

/**
 * The front picture: how long each theme shows, and the steps of the wipe to the next one (about a
 * second in all).
 */
export const HERO = { holdMs: 2800, steps: 12, stepMs: 80 } as const;
