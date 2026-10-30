import type { ReactNode } from "react";

import { TitleInBar } from "./title-bar.ts";

/** What is inside has a top bar that carries the title of the page: a card then leaves its own out. */
export function TitleCarriedByBar({ children }: { readonly children: ReactNode }) {
  return <TitleInBar value>{children}</TitleInBar>;
}
