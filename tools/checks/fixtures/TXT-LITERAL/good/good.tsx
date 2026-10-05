import { t } from "@bugaboo/messages";

export function Good() {
  return (
    <section aria-label={t("shell.menu")} title={t("common.close")}>
      <p>{t("home.intro")}</p>
      <input placeholder={t("search.placeholder")} type="radio" />
    </section>
  );
}
