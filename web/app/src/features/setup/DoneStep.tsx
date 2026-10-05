import { t } from "@bugaboo/messages";
import { Button, Card } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";

import { paths } from "../../shared/paths.ts";
import styles from "./Setup.module.css";

/** The last step: the account is ready. */
export function DoneStep() {
  const navigate = useNavigate();
  return (
    <Card heading={t("first-run.done.title")} level={1}>
      <p className={styles.text}>{t("first-run.done.text")}</p>
      <Button
        primary
        wide
        onPress={() => {
          void navigate({ to: paths.home });
        }}
      >
        {t("first-run.done.go")}
      </Button>
    </Card>
  );
}
