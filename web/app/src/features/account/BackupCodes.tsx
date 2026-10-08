import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, CopyButton, partOf, part } from "@bugaboo/ui";

export interface BackupCodesProps {
  /** The codes, in the form that is shown to the person. */
  readonly codes: readonly string[];
  /** The person says that the codes are saved; they are not shown again. */
  readonly onSaved: () => void;
}

/** Backup codes, shown once: the server keeps only a hash of each. */
export function BackupCodes({ codes, onSaved }: BackupCodesProps) {
  return (
    <div {...partOf(part.stack)}>
      <Alert tone={alertTone.warning}>{t("backup.shown-once")}</Alert>
      <ul {...partOf(part.accountCodes)} aria-label={t("backup.list")}>
        {codes.map((code) => (
          <li key={code}>{code}</li>
        ))}
      </ul>
      <div {...partOf(part.row)}>
        <CopyButton
          text={codes.join("\n")}
          label={t("backup.copy")}
          copiedLabel={t("backup.copied")}
        />
        <Button look={buttonLook.primary} onPress={onSaved}>
          {t("backup.saved")}
        </Button>
      </div>
    </div>
  );
}
