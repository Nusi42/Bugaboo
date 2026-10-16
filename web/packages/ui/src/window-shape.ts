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
