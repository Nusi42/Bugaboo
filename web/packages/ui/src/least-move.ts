/**
 * A difference of more than half a pixel is a move (UX §6.4: not even 1 px). Less is the subpixel layout of the
 * browser, which nobody sees.
 */
export const LEAST_MOVE_PX = 0.5;
