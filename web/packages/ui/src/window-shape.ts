/** What a window is doing besides being open: parked as a bar, or filling the page. */
export interface WindowShape {
  readonly minimized: boolean;
  readonly maximized: boolean;
}

/** What can happen to the shape of a window. */
export const shapeAction = {
  minimize: "minimize",
  restore: "restore",
  toggleMaximize: "toggle-maximize",
  closed: "closed",
} as const;
export type ShapeAction = (typeof shapeAction)[keyof typeof shapeAction];

/** A window as it is first drawn: neither minimized nor maximized. */
export const firstShape: WindowShape = { minimized: false, maximized: false };

/** What each thing that can happen does to the shape. A new action is a type error until it is here. */
const next: Readonly<Record<ShapeAction, (shape: WindowShape) => WindowShape>> = {
  minimize: (shape) => ({ ...shape, minimized: true }),
  restore: (shape) => ({ ...shape, minimized: false }),
  "toggle-maximize": (shape) => ({ ...shape, maximized: !shape.maximized }),
  closed: () => firstShape,
};

/** The shape after something happened. A window that closes opens again as it was first drawn. */
export function reduceShape(shape: WindowShape, action: ShapeAction): WindowShape {
  return next[action](shape);
}

/** Whether a window is on the screen, and where it stands while its content arrives. */
export interface WindowPresence {
  /** Its content arrived while it was open; it stays shown until it closes. */
  readonly shown: boolean;
  /** It was shown loading, at the top of the page; it keeps that top until it closes. */
  readonly anchored: boolean;
}

/** A window that is closed, or open with its content still on the way. */
export const hiddenPresence: WindowPresence = { shown: false, anchored: false };

/** What is known about a window now. */
export interface PresenceFacts {
  readonly isOpen: boolean;
  /** Its content has arrived. */
  readonly ready: boolean;
  /** Its content has been on the way for longer than grey shapes wait (UX §6.1). */
  readonly late: boolean;
}

/**
 * Where a window is after what is known now. It is shown once its content is ready and stays shown
 * while it is open, so content that arrives later never hides it again. One that had to show grey
 * shapes stands at the top, and keeps that top when its content arrives: in the middle it would jump.
 */
export function presenceAfter(previous: WindowPresence, facts: PresenceFacts): WindowPresence {
  if (!facts.isOpen) {
    return hiddenPresence;
  }
  return {
    shown: previous.shown || facts.ready,
    anchored: previous.anchored || (!previous.shown && facts.late),
  };
}

/**
 * How a window stands: `quiet` (its content is on the way and nothing shows yet), `loading` (grey
 * shapes at the top), `top` (its content, where the grey shapes stood) or `middle` (its content, in
 * the middle of the page).
 */
export const windowPhase = {
  quiet: "quiet",
  loading: "loading",
  top: "top",
  middle: "middle",
} as const;
export type WindowPhase = (typeof windowPhase)[keyof typeof windowPhase];

/** The phase of a window from its presence and whether its content is late. */
export function phaseOf(presence: WindowPresence, late: boolean): WindowPhase {
  if (!presence.shown) {
    return late ? windowPhase.loading : windowPhase.quiet;
  }
  return presence.anchored ? windowPhase.top : windowPhase.middle;
}
