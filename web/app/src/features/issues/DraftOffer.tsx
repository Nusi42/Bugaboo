import { t } from "@bugaboo/messages";
import { Alert, Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { formatAgo } from "../../shared/format.ts";
import type { FormDraft } from "./find-new-issue-draft.ts";

export interface DraftOfferProps {
  readonly draft: FormDraft;
  readonly now: number;
  readonly onContinue: () => void;
  readonly onDrop: () => void;
}

/** *Continue your draft from yesterday* (ISS-11): the form opened and the person has a draft of it. */
export function DraftOffer({ draft, now, onContinue, onDrop }: DraftOfferProps) {
  const when = formatAgo(draft.savedAt, now);
  return (
    <Alert>
      <div {...partOf(part.issueNote)}>
        <span>{t("issues.draft-found-form", { when })}</span>
        <Button look={buttonLook.primary} onPress={onContinue}>
          {t("issues.draft-continue", { when })}
        </Button>
        <Button onPress={onDrop}>{t("issues.draft-drop")}</Button>
      </div>
    </Alert>
  );
}
