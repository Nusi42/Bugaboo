import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { useLookAtWork, type Work } from "./agent-work.ts";

/** Work that a result was accepted into by itself, and nobody looked at yet. */
export function AcceptedByItself({ work }: { readonly work: Work }) {
  const looked = useLookAtWork();
  if (!work.accepted_by_itself) {
    return null;
  }
  return (
    <div {...partOf(part.agentWorkSection)}>
      <Alert tone={alertTone.info}>{t("agent-work.accepted-by-itself")}</Alert>
      {work.choose_next_status ? (
        <p {...partOf(part.muted)}>{t("agent-work.choose-next-status")}</p>
      ) : null}
      {looked.isError ? <Alert tone={alertTone.danger}>{messageOf(looked.error)}</Alert> : null}
      <div {...partOf(part.agentWorkActions)}>
        <Button
          isPending={looked.isPending}
          onPress={() => {
            looked.mutate({ params: { path: { work: work.id } } });
          }}
        >
          {t("agent-work.looked-at")}
        </Button>
      </div>
    </div>
  );
}
