import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, Card, CopyButton, partOf, part } from "@bugaboo/ui";

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
      <div {...partOf(part.stack)}>
        <Alert tone={alertTone.warning}>{t("access-keys.shown-once")}</Alert>
        <p {...partOf(part.muted)}>{t("access-keys.key-label")}</p>
        <code {...partOf(part.accessKeysKey)}>{secret}</code>
        <div {...partOf(part.row, { variants: ["stretch"] })}>
          <CopyButton
            text={secret}
            label={t("access-keys.copy")}
            copiedLabel={t("access-keys.copied")}
          />
          <Button look={buttonLook.primary} onPress={onDone}>
            {t("access-keys.done")}
          </Button>
        </div>
      </div>
    </Card>
  );
}
