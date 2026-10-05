import { t } from "@bugaboo/messages";
import { useEffect } from "react";

/** Puts the name of the page in the title of the browser tab, where screen readers announce it. */
export function usePageTitle(title: string): void {
  useEffect(() => {
    document.title = `${title} · ${t("app.name")}`;
  }, [title]);
}
