import type { ReactNode } from "react";

import { TitleInBar } from "./title-bar.ts";

/** What is inside says it has a top bar that carries the title of the page: a card then leaves its own out. */
export function TitleCarriedByBar({
  carried,
  children,
}: {
  readonly carried: boolean;
  readonly children: ReactNode;
}) {
  return <TitleInBar value={carried}>{children}</TitleInBar>;
}
