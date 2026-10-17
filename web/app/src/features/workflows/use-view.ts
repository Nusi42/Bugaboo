import { useLayout } from "@bugaboo/ui";
import { useState } from "react";

/** The two views of a workflow: the plain list, and the drawing of boxes and arrows. */
export type ViewWord = "list" | "drawing";

/** Which view is shown, and whether the drawing is on offer: a phone shows only the list. */
export interface View {
  readonly drawing: boolean;
  readonly canDraw: boolean;
  readonly choose: (word: ViewWord) => void;
}

/** The view of the editor: the list by default; a computer may also draw the workflow. */
export function useView(): View {
  const canDraw = useLayout() === "computer";
  const [word, choose] = useState<ViewWord>("list");
  return { drawing: canDraw && word === "drawing", canDraw, choose };
}
