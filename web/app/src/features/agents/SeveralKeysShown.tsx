import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, Card, CopyButton, partOf, part } from "@bugaboo/ui";

import { allKeysText, keyLine, type CreatedAgent } from "./several-agents.ts";

export interface SeveralKeysShownProps {
  readonly created: readonly CreatedAgent[];
  /** The person says that they copied the keys: they are not shown again. */
  readonly onDone: () => void;
}

/** The keys of the agents a list created, shown once, each with its own copy and one copy of all. */
export function SeveralKeysShown({ created, onDone }: SeveralKeysShownProps) {
  return (
    <Card heading={t("agents.several-shown-title", { count: created.length })}>
      <div {...partOf(part.stack)}>
        <Alert tone={alertTone.warning}>{t("agents.several-shown-once")}</Alert>
        <ul {...partOf(part.list)} aria-label={t("agents.several-keys")}>
          {created.map((item) => (
            <li key={item.key.key.id} {...partOf(part.stack)}>
              <p {...partOf(part.muted)}>{item.agent.name}</p>
              <code {...partOf(part.keyBox)}>{item.key.secret}</code>
              <div {...partOf(part.row)}>
                <CopyButton
                  text={keyLine(item)}
                  label={t("agents.several-copy-one", { name: item.agent.name ?? "" })}
                  copiedLabel={t("access-keys.copied")}
                />
              </div>
            </li>
          ))}
        </ul>
        <div {...partOf(part.row, { variants: ["stretch"] })}>
          <CopyButton
            text={allKeysText(created)}
            label={t("agents.several-copy-all")}
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
