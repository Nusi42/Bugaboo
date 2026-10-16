import { partWords } from "./tokens.ts";

/** How big a window is, in pixels. */
export interface WindowSize {
  readonly width: number;
  readonly height: number;
}

/** What a window needs from its element: its size on the page, and a way to set its variables. */
export interface WindowElement {
  readonly getBoundingClientRect: () => Readonly<Pick<DOMRect, "width" | "height">>;
  readonly style: Readonly<Pick<CSSStyleDeclaration, "setProperty">>;
}

/** A place to drag: the four edges and the four corners, as the theme file names them. */
export type WindowEdge = (typeof partWords)["window.handle"]["variant"][number];

/** All the places to drag, in the order the theme file names them. */
export const windowEdges: readonly WindowEdge[] = partWords["window.handle"].variant;

/** Which way an edge moves a size: `1` when the pointer going right (or down) makes it bigger. */
interface Sides {
  readonly x: -1 | 0 | 1;
  readonly y: -1 | 0 | 1;
}

/** The sides an edge or a corner drags. A place the theme file has and this table lacks is a type error. */
const sides = {
  e: { x: 1, y: 0 },
  w: { x: -1, y: 0 },
  s: { x: 0, y: 1 },
  n: { x: 0, y: -1 },
  se: { x: 1, y: 1 },
  nw: { x: -1, y: -1 },
  ne: { x: 1, y: -1 },
  sw: { x: -1, y: 1 },
} as const satisfies Readonly<Record<WindowEdge, Sides>>;

/**
 * The size after an edge was dragged by `by` pixels from where the drag began. The window stays in
 * the middle of the page, so it grows on both sides and the edge that is dragged follows the
 * pointer. The smallest and the largest size are the theme file's (`min-width` and `max-width` of
 * the window), so a size too small is shown at the smallest and the size the person ends with is
 * read back from the window.
 */
export function resizedBy(
  from: WindowSize,
  edge: WindowEdge,
  by: { readonly x: number; readonly y: number },
): WindowSize {
  const { x, y } = sides[edge];
  return { width: from.width + 2 * x * by.x, height: from.height + 2 * y * by.y };
}

/** How far an arrow key changes the size, in pixels (UX §6.3, UX-04). */
export const keyboardStep = 24;

/** The keys of the keyboard (the standard's names), not text for people. */
const arrow = {
  left: "ArrowLeft",
  right: "ArrowRight",
  up: "ArrowUp",
  down: "ArrowDown",
} as const;

/** How an arrow key changes the size: wider, narrower, taller or shorter. */
const change: Readonly<Record<string, WindowSize>> = {
  [arrow.right]: { width: keyboardStep, height: 0 },
  [arrow.left]: { width: -keyboardStep, height: 0 },
  [arrow.down]: { width: 0, height: keyboardStep },
  [arrow.up]: { width: 0, height: -keyboardStep },
};

/** The size after an arrow key; nothing for any other key. */
export function sizeAfterKey(from: WindowSize, key: string): WindowSize | undefined {
  const by = change[key];
  return by === undefined
    ? undefined
    : { width: from.width + by.width, height: from.height + by.height };
}

function isSize(value: unknown): value is WindowSize {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const { width, height } = value as Partial<Record<keyof WindowSize, unknown>>;
  return typeof width === "number" && typeof height === "number" && width > 0 && height > 0;
}

/** The sizes of the forms a person resized, by the name of the form. */
export type WindowSizes = Readonly<Record<string, WindowSize>>;

/** Reads the setting that holds the sizes (one JSON object); what is not a size is left out. */
export function parseWindowSizes(text: string): WindowSizes {
  try {
    const parsed: unknown = JSON.parse(text);
    if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
      return {};
    }
    const sizes: Record<string, WindowSize> = {};
    for (const [form, value] of Object.entries(parsed)) {
      if (isSize(value)) {
        sizes[form] = value;
      }
    }
    return sizes;
  } catch {
    return {};
  }
}
