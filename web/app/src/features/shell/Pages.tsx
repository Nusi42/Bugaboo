import { t } from "@bugaboo/messages";
import { Alert, Button, Card, Link } from "@bugaboo/ui";

import { paths } from "../../shared/paths.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { Frame } from "./Frame.tsx";
import styles from "./Shell.module.css";

/** The brand, for the pages that stand outside the layouts. */
function Brand() {
  return <span className={styles.brand}>{t("app.name")}</span>;
}

/** An address that no page has. */
export function NotFoundPage() {
  usePageTitle(t("page.not-found.title"));
  return (
    <Frame header={<Brand />}>
      <div className={styles.narrow}>
        <Card heading={t("page.not-found.title")} level={1}>
          <p>{t("page.not-found.text")}</p>
          <Link href={paths.home}>{t("page.not-found.home")}</Link>
        </Card>
      </div>
    </Frame>
  );
}

/** Something broke while a page was being made: nothing the person did. */
export function ErrorPage() {
  usePageTitle(t("error.server.internal"));
  return (
    <Frame header={<Brand />}>
      <div className={styles.narrow}>
        <Card heading={t("app.name")} level={1}>
          <Alert danger>{t("error.server.internal")}</Alert>
          <Button
            primary
            onPress={() => {
              window.location.reload();
            }}
          >
            {t("common.try-again")}
          </Button>
        </Card>
      </div>
    </Frame>
  );
}

/** What shows while a page is being got ready, if that takes a moment. */
export function PendingPage() {
  return (
    <Frame header={<Brand />}>
      <p>{t("common.loading")}</p>
    </Frame>
  );
}
