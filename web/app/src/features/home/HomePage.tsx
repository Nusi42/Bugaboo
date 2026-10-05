import { t } from "@bugaboo/messages";
import { Card } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { meOptions } from "../../shared/session.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import styles from "./Home.module.css";

/** The start page of somebody who is signed in. The rest of Bugaboo arrives with later versions. */
export function HomePage() {
  const me = useQuery(meOptions()).data;
  const title = t("home.title", { name: me?.display_name ?? "" });
  usePageTitle(title);
  return (
    <Card heading={title} level={1}>
      <p className={styles.text}>{t("home.intro")}</p>
      {me?.administrator === true ? <p className={styles.text}>{t("home.admin")}</p> : null}
    </Card>
  );
}
