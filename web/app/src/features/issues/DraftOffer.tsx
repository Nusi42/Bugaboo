import { t } from "@bugaboo/messages";

import { DraftOffered } from "../../shared/editing/DraftOffered.tsx";
import type { FormDraft } from "./find-new-issue-draft.ts";

export interface DraftOfferProps {
  readonly draft: FormDraft;
  readonly now: number;
  readonly onContinue: () => void;
  readonly onDrop: () => void;
}

/** The offer of the draft of a new issue (ISS-11): the form opened and the person has a draft of it. */
export function DraftOffer({ draft, now, onContinue, onDrop }: DraftOfferProps) {
  return (
    <DraftOffered
      found={(when) => t("issues.draft-found-form", { when })}
      savedAt={draft.savedAt}
      now={now}
      onContinue={onContinue}
      onDrop={onDrop}
    />
  );
}
