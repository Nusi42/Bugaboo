import { createContext, useContext } from "react";

/** Whether a top bar above the page says its title; a page without one (the sign-in) shows its own. */
export const TitleInBar = createContext(false);

/** Whether the title of the page is in a top bar above it. */
export function useTitleInBar(): boolean {
  return useContext(TitleInBar);
}
