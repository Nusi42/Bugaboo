import { t } from "@bugaboo/messages";
import { Alert, Button, CopyButton } from "@bugaboo/ui";

import styles from "./Account.module.css";

export interface BackupCodesProps {
  /** The codes, in the form that is shown to the person. */
  readonly codes: readonly string[];
  /** The person says that the codes are saved; they are not shown again. */
  readonly onSaved: () => void;
}

/** Backup codes, shown once: the server keeps only a hash of each. */
export function BackupCodes({ codes, onSaved }: BackupCodesProps) {
  return (
    <div className={styles.stack}>
      <Alert warning>{t("backup.shown-once")}</Alert>
      <ul className={styles.codes} aria-label={t("backup.list")}>
        {codes.map((code) => (
          <li key={code}>{code}</li>
        ))}
      </ul>
      <div className={styles.row}>
        <CopyButton
          text={codes.join("\n")}
          label={t("backup.copy")}
          copiedLabel={t("backup.copied")}
        />
        <Button primary onPress={onSaved}>
          {t("backup.saved")}
        </Button>
      </div>
    </div>
  );
}
