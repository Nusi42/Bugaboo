import { t } from "@bugaboo/messages";
import { Alert, alertTone, ConfirmDialog, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { formatDateTime } from "../../shared/format.ts";
import { personName } from "../../shared/person.ts";
import { providerName, useConfirmAgreement, type AiSetup } from "./ai.ts";

/**
 * The agreement step for an outside provider (PRIV-08, GDPR Art. 28): the data goes to a provider
 * outside the organization, so an administrator confirms first that a data processing agreement is
 * in place. It is kept with who confirmed it and when, and a change of provider asks again.
 */
export function AiAgreement({ setup }: { readonly setup: AiSetup }) {
  const confirm = useConfirmAgreement();
  const destination = setup.destination;
  if (
    setup.agreement_status === "not-needed" ||
    destination === undefined ||
    destination === null
  ) {
    return <p {...partOf(part.muted)}>{t("ai-setup.agreement-not-needed")}</p>;
  }
  const provider = providerName(destination);
  const confirmed = setup.agreement;
  return (
    <div {...partOf(part.stack)}>
      <p {...partOf(part.muted)}>{t("ai-setup.agreement-text", { provider })}</p>
      {setup.agreement_status === "confirmed" && confirmed !== undefined && confirmed !== null ? (
        <Alert tone={alertTone.success}>
          {t("ai-setup.agreement-confirmed", {
            person: personName(confirmed.confirmed_by),
            at: formatDateTime(confirmed.confirmed_at),
          })}
        </Alert>
      ) : (
        <div {...partOf(part.row)}>
          <ConfirmDialog
            triggerLabel={t("ai-setup.agreement-confirm")}
            title={t("ai-setup.agreement-title", { provider })}
            text={t("ai-setup.agreement-sentence", { provider })}
            confirmLabel={t("ai-setup.agreement-yes")}
            cancelLabel={t("common.cancel")}
            onConfirm={() => {
              confirm.mutate({ body: { destination } });
            }}
          />
        </div>
      )}
      {confirm.isError ? <Alert tone={alertTone.danger}>{messageOf(confirm.error)}</Alert> : null}
    </div>
  );
}
