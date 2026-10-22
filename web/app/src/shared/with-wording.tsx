import { t } from "@bugaboo/messages";
import { Skeleton } from "@bugaboo/ui";
import type { Wording } from "@bugaboo/wasm";
import type { ReactNode } from "react";

import { useWording } from "./use-wording.ts";

/**
 * Words that the shared wording of `bb-core` chooses (a sentence of a change or of a
 * notification). The module comes from `bb-wasm` the first time a sentence needs it, so a
 * skeleton stands in until then.
 */
export function WithWording({ children }: { readonly children: (wording: Wording) => ReactNode }) {
  const loaded = useWording();
  if (loaded.state === "loading") {
    return <Skeleton />;
  }
  return loaded.state === "failed" ? (
    <>{t("common.words-failed")}</>
  ) : (
    <>{children(loaded.wording)}</>
  );
}
