import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import type { AssistantStatus } from "./assistant.ts";

/**
 * Where the answers come from and whether people go by name (UX §4.12, COMP-08), always on top of the
 * panel. The provider and its place are the server's word for where the data goes; the sentences
 * are the catalog's, so no provider is named here.
 */
export function WhereFrom({ status }: { readonly status: AssistantStatus }) {
  const destination = status.destination;
  return (
    <div {...partOf(part.stack)}>
      {destination === "anthropic" ? <p>{t("assistant.source-anthropic")}</p> : null}
      {destination === "other-provider" ? <p>{t("assistant.source-other-provider")}</p> : null}
      {destination === "local-model" ? <p>{t("assistant.source-local-model")}</p> : null}
      <p {...partOf(part.muted)}>
        {destination === "local-model"
          ? t("assistant.names-as-seen")
          : status.names_sent
            ? t("assistant.names-sent")
            : t("assistant.names-role")}
      </p>
    </div>
  );
}
