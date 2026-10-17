import { t } from "@bugaboo/messages";

/** What a line says about a place: offered or not. The one wording, for every screen of the offers. */
export function offerStateName(offered: boolean | null | undefined): string {
  return offered === true ? t("projects.offer-state-offered") : t("projects.offer-state-not");
}
