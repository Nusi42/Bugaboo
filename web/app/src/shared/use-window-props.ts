import type { WindowProps } from "@bugaboo/ui";

import { useWindowSize } from "./use-window-size.ts";
import type { WindowForm } from "./window-forms.ts";
import { windowLabels } from "./window-labels.ts";

/**
 * What every window of a form is given besides its title and its content: the words of its
 * buttons, the size the person gave it before, and the way to remember a new size (UX §6.3).
 */
export function useWindowProps(
  form: WindowForm,
): Pick<WindowProps, "labels" | "size" | "onResized"> {
  const { size, remember } = useWindowSize(form);
  return { labels: windowLabels(), size, onResized: remember };
}
