import { t } from "@bugaboo/messages";

import { DraftOffered } from "../../shared/editing/DraftOffered.tsx";
import { editedThing } from "../../shared/editing/edited-thing.ts";
import { useNow } from "../../shared/use-now.ts";
import type { Description } from "./use-description.ts";

/** A draft that was found when the box opened: continue it or throw it away. */
export function Offered({ description }: { readonly description: Description }) {
  const { offered } = description;
  const now = useNow();
  if (offered === null) {
    return null;
  }
  return (
    <DraftOffered
      found={(when) => t("editing.draft-found", { thing: t(editedThing.description), when })}
      savedAt={offered.savedAt}
      now={now}
      onContinue={description.continueDraft}
      onDrop={description.dropDraft}
    />
  );
}
