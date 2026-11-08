import { t } from "@bugaboo/messages";
import { NameValueList, NameValueRow, partOf, part } from "@bugaboo/ui";

import { QueryGate } from "../shell/index.ts";
import { countryName, providerName, useWhatGoesWhere, type WhatGoesWhere } from "./ai.ts";

/** The projects that switched AI off, in one line, and a word when the list was cut. */
function switchedOffText(shown: Readonly<WhatGoesWhere>): string {
  if (shown.switched_off.length === 0) {
    return t("ai-setup.where-none-off");
  }
  const names = shown.switched_off.map((project) => project.name).join(", ");
  return shown.truncated ? t("ai-setup.where-off-more", { names }) : names;
}

/**
 * *What goes where* (PRIV-08, COMP-08): the provider and its country, the model, which data may go
 * and which projects keep AI off. The provider and the country are the catalog's words for where
 * the data goes, so the browser names no provider on its own.
 */
export function AiWhatGoesWhere() {
  const found = useWhatGoesWhere();
  return (
    <QueryGate error={found.error} data={found.data} loading={null}>
      {(shown) => (
        <div {...partOf(part.stack)}>
          <NameValueList>
            <NameValueRow name={t("ai-setup.where-provider")}>
              {providerName(shown.destination)}
            </NameValueRow>
            <NameValueRow name={t("ai-setup.where-country")}>
              {countryName(shown.destination)}
            </NameValueRow>
            <NameValueRow name={t("ai-setup.where-model")}>
              {shown.model === "" ? t("common.nothing") : shown.model}
            </NameValueRow>
            <NameValueRow name={t("ai-setup.where-state")}>
              {shown.enabled ? t("common.on") : t("common.off")}
            </NameValueRow>
            <NameValueRow name={t("ai-setup.where-people")}>
              {shown.names_sent ? t("ai-setup.where-names") : t("ai-setup.where-roles")}
            </NameValueRow>
            <NameValueRow name={t("ai-setup.where-off")}>{switchedOffText(shown)}</NameValueRow>
          </NameValueList>
          <p {...partOf(part.muted)}>{t("ai-setup.where-note")}</p>
        </div>
      )}
    </QueryGate>
  );
}
