import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, Skeleton } from "@bugaboo/ui";
import type { Wording } from "@bugaboo/wasm";
import type { ReactNode } from "react";

import { retryWording, useWording } from "./use-wording.ts";

/**
 * Words that the shared wording of `bb-core` chooses (a sentence of a change or of a
 * notification). The module comes from `bb-wasm` the first time a sentence needs it, so a
 * skeleton stands in until then. Where it could not be loaded the sentence is left out: the
 * block says so once (`WordingNotice`).
 */
export function WithWording({ children }: { readonly children: (wording: Wording) => ReactNode }) {
  const loaded = useWording();
  if (loaded.state === "loading") {
    return <Skeleton />;
  }
  return loaded.state === "failed" ? null : <>{children(loaded.wording)}</>;
}

/**
 * Says once for a block that its words could not be loaded, with *Try again*. Nothing while they
 * load or are there.
 */
export function WordingNotice() {
  const loaded = useWording();
  if (loaded.state !== "failed") {
    return null;
  }
  return (
    <Alert tone={alertTone.danger}>
      {t("common.words-failed")} <Button onPress={retryWording}>{t("common.try-again")}</Button>
    </Alert>
  );
}
