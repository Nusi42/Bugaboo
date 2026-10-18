import { createContext } from "react";

/**
 * The place under the content of a phone where the bar of an open issue stands (none on a computer
 * or a tablet): a part of the column of the app, so the bar never covers the page.
 */
export const BarSlot = createContext<HTMLElement | null>(null);
