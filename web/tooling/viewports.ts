import { breakpoints } from "../packages/ui/src/tokens.ts";

/**
 * The windows the browser tests use, by the layout they show. The widths that decide the layout
 * (`breakpoints`) come from the design tokens; the others are chosen to lie inside their layout.
 */
export const viewports = {
  /** As narrow as a computer layout can be. */
  computer: { width: breakpoints.computer, height: 800 },
  /** Between the two breakpoints: the sidebar is a menu that slides in. */
  tablet: { width: 900, height: 1000 },
  /** A tablet turned sideways: wider than tall, and still narrower than a computer. */
  tabletSideways: { width: breakpoints.computer - 1, height: 800 },
  /** A small phone. */
  phone: { width: 360, height: 740 },
  /** The phone of the pictures for the review. */
  phoneForPictures: { width: 400, height: 860 },
} as const;
