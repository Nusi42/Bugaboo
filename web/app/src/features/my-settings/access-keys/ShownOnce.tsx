import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, Card, CopyButton, partOf, part } from "@bugaboo/ui";

export interface ShownOnceProps {
  /** The whole key. Bugaboo cannot show it again. */
  readonly secret: string;
  /** The person says that they copied it: it is not shown again. */
  readonly onDone: () => void;
  /** What the card is called; *Your new access key* without it (an agent key has its own). */
  readonly title?: string;
  /** What the key is called above it; *The access key* without it (*The agent key* for an agent). */
  readonly label?: string;
}

/** The new key, shown once, with a button that copies it. */
export function ShownOnce({ secret, onDone, title, label }: ShownOnceProps) {
  return (
    <Card heading={title ?? t("access-keys.shown-title")}>
      <div {...partOf(part.stack)}>
        <Alert tone={alertTone.warning}>{t("access-keys.shown-once")}</Alert>
        <p {...partOf(part.muted)}>{label ?? t("access-keys.key-label")}</p>
        <code {...partOf(part.keyBox)}>{secret}</code>
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
