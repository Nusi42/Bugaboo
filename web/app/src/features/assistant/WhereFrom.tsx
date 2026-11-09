import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import type { AssistantStatus } from "./assistant.ts";

/**
 * Where the answers come from and whether people go by name (UX §4.12, COMP-08), always on top of the
 * panel. The provider and its place are the server's word for where the data goes; the sentences
 * are the catalog's, so no provider is named here.
 */
export function WhereFrom({ status, source = true, names = true }: WhereFromProps) {
  const destination = status.destination;
  return (
    <div {...partOf(part.column, { variants: ["snug"] })}>
      {source ? <Source destination={destination} /> : null}
      {names ? (
        <p {...partOf(part.muted)}>
          {destination === "local-model"
            ? t("assistant.names-as-seen")
            : status.names_sent
              ? t("assistant.names-sent")
              : t("assistant.names-role")}
        </p>
      ) : null}
    </div>
  );
}

export interface WhereFromProps {
  readonly status: AssistantStatus;
  /** Says where the answers come from; leave it out where that stands elsewhere. */
  readonly source?: boolean;
  /** Says whether people go by name; leave it out where that stands elsewhere. */
  readonly names?: boolean;
}

/** The sentence that says where the answers come from. */
function Source({ destination }: { readonly destination: AssistantStatus["destination"] }) {
  if (destination === "anthropic") {
    return <p>{t("assistant.source-anthropic")}</p>;
  }
  if (destination === "other-provider") {
    return <p>{t("assistant.source-other-provider")}</p>;
  }
  if (destination === "local-model") {
    return <p>{t("assistant.source-local-model")}</p>;
  }
  return null;
}
