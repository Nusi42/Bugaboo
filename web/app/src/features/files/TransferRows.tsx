import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { useFileLimit } from "./files.ts";
import { formatSize } from "./size.ts";
import { transfers, type Transfer } from "./transfers.ts";

/** A file that waits for its turn or is on its way: its name, and the bar where it will be. */
function Sending({ transfer }: { readonly transfer: Transfer }) {
  return (
    <div {...partOf(part.filesRow)}>
      <div {...partOf(part.filesInfo)}>
        <span {...partOf(part.filesName)}>{transfer.name}</span>
        {transfer.status === "waiting" ? (
          <span {...partOf(part.filesMeta)}>{t("files.waiting", { name: transfer.name })}</span>
        ) : (
          <div {...partOf(part.row)}>
            <progress
              {...partOf(part.filesProgress)}
              aria-label={t("files.sending", { name: transfer.name })}
              value={transfer.sent}
              max={transfer.size}
            />
            <span {...partOf(part.filesMeta)}>
              {t("files.sent-of", {
                sent: formatSize(transfer.sent),
                size: formatSize(transfer.size),
              })}
            </span>
          </div>
        )}
      </div>
    </div>
  );
}

/** A file that was not added: the words that say why, and what to do about it. */
function NotAdded({ transfer }: { readonly transfer: Transfer }) {
  // The limit is read now, so the sentence names the limit the project has when it is shown.
  const limit = useFileLimit(transfer.target.issue);
  const reason = transfer.tooBig
    ? t("files.too-big", { limit: formatSize(limit) })
    : (transfer.message ?? "");
  return (
    <Alert tone={alertTone.danger}>
      <div {...partOf(part.filesRefusal)}>
        <span>
          {t("files.not-added", {
            name: transfer.name,
            reason,
          })}
        </span>
        <div {...partOf(part.filesActions)}>
          {transfer.canRetry ? (
            <Button
              look={buttonLook.plain}
              onPress={() => {
                transfers.retry(transfer.id);
              }}
            >
              {t("common.try-again")}
            </Button>
          ) : null}
          <Button
            look={buttonLook.plain}
            onPress={() => {
              transfers.dismiss(transfer.id);
            }}
          >
            {t("common.close")}
          </Button>
        </div>
      </div>
    </Alert>
  );
}

/** The files on their way to one place, in the order they were chosen. */
export function TransferRows({ items }: { readonly items: readonly Transfer[] }) {
  if (items.length === 0) {
    return null;
  }
  return (
    <ul {...partOf(part.list)}>
      {items.map((transfer) => (
        <li key={transfer.id}>
          {transfer.status === "refused" ? (
            <NotAdded transfer={transfer} />
          ) : (
            <Sending transfer={transfer} />
          )}
        </li>
      ))}
    </ul>
  );
}
