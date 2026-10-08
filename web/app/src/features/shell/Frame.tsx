import { t } from "@bugaboo/messages";
import type { ReactNode } from "react";

import { usePageFocus } from "../../shared/focus.ts";
import { CONTENT_HREF, CONTENT_ID } from "./content-id.ts";
import { ThemeBar } from "./ThemeBar.tsx";
import { useAppearance, partOf, part } from "@bugaboo/ui";

export interface FrameProps {
  /** The top of the page: the name of the product. */
  readonly header: ReactNode;
  readonly children: ReactNode;
}

/**
 * The frame of the pages before sign-in: skip link, header, the content, and the choice of theme.
 * (The signed-in app has its own frame, `AppLayout`.) When the person goes to another page, the
 * focus moves to its heading.
 */
export function Frame({ header, children }: FrameProps) {
  usePageFocus(CONTENT_ID);
  useAppearance();
  return (
    <div {...partOf(part.shellPage)}>
      <a {...partOf(part.skip)} href={CONTENT_HREF}>
        {t("shell.skip")}
      </a>
      <header {...partOf(part.shellHeader)}>{header}</header>
      <main id={CONTENT_ID} {...partOf(part.shellContent)} tabIndex={-1}>
        {children}
      </main>
      <footer {...partOf(part.shellFooter)}>
        <ThemeBar />
      </footer>
    </div>
  );
}
