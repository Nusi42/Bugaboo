import { t } from "@bugaboo/messages";

import { useIssueTakesScreen } from "./use-issue-takes-screen.ts";

/**
 * What the close button of the side panel is called. Where the issue takes the whole screen it
 * leads back to the list; elsewhere the panel's own name stands.
 */
export function useCloseLabel(): string | undefined {
  return useIssueTakesScreen().whole ? t("focus.back") : undefined;
}
