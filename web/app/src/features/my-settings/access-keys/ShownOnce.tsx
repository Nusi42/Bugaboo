import { t } from "@bugaboo/messages";
import { Alert, Button, Card, CopyButton } from "@bugaboo/ui";

import styles from "./AccessKeys.module.css";

export interface ShownOnceProps {
  /** The whole key. Bugaboo cannot show it again. */
  readonly secret: string;
  /** The person says that they copied it: it is not shown again. */
  readonly onDone: () => void;
}

/** The new key, shown once, with a button that copies it. */
export function ShownOnce({ secret, onDone }: ShownOnceProps) {
  return (
    <Card heading={t("access-keys.shown-title")}>
      <div className={styles.stack}>
        <Alert warning>{t("access-keys.shown-once")}</Alert>
        <p className={styles.muted}>{t("access-keys.key-label")}</p>
        <code className={styles.key}>{secret}</code>
        <div className={styles.row}>
          <CopyButton
            text={secret}
            label={t("access-keys.copy")}
            copiedLabel={t("access-keys.copied")}
          />
          <Button primary onPress={onDone}>
            {t("access-keys.done")}
          </Button>
        </div>
      </div>
    </Card>
  );
}
