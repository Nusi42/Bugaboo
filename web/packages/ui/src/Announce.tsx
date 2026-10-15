import type { ReactNode } from "react";
import { VisuallyHidden } from "react-aria-components";

/**
 * Words that screen readers say when they change and that nobody needs to read on the screen (a
 * count, *Address copied*). The element stays in the page, so a change is heard.
 */
export function Announce({ children }: { readonly children: ReactNode }) {
  return (
    <VisuallyHidden>
      <output>{children}</output>
    </VisuallyHidden>
  );
}
