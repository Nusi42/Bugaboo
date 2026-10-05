import { t } from "@bugaboo/messages";
import { Card } from "@bugaboo/ui";

import { useMe } from "../../shared/use-me.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import styles from "./Home.module.css";

/** The start page of somebody who is signed in. The rest of Bugaboo arrives with later versions. */
export function HomePage() {
  const me = useMe();
  const title = t("home.title", { name: me?.display_name ?? "" });
  usePageTitle(title);
  return (
    <Card heading={title} level={1}>
      <p className={styles.text}>{t("home.intro")}</p>
      {me !== undefined && me.permissions.length > 0 ? (
        <p className={styles.text}>{t("home.admin")}</p>
      ) : null}
    </Card>
  );
}
