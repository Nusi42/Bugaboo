import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, Card, part, partOf } from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { useRegion } from "../../shared/region.ts";
import {
  SearchMailWindow,
  useSearchMails,
  useSetSearchMail,
  useStopSearchMail,
  whenText,
  type SearchMail,
} from "../search/index.ts";

/** What the line under the name says: when it comes, or why it sends nothing. */
function lineOf(mail: SearchMail, when: string): string {
  if (mail.paused === "no-longer-opens") {
    return t("search.mails-no-longer-opens");
  }
  return mail.paused === "switched-off" ? t("search.mails-switched-off") : when;
}

/** One e-mailed search: its name, when it comes or why it waits, *Change* and *Stop*. */
function MailRow({ mail, onStop }: { readonly mail: SearchMail; readonly onStop: () => void }) {
  const region = useRegion();
  const [changing, setChanging] = useState(false);
  const name = mail.name ?? t("search.mails-lost-search");
  return (
    <li {...partOf(part.item, { variants: ["row"] })}>
      <span {...partOf(part.stack)}>
        <span>{name}</span>
        <span {...partOf(part.muted)}>{lineOf(mail, whenText(mail, region))}</span>
      </span>
      <span {...partOf(part.itemActions)}>
        {mail.name === null ? null : (
          <>
            <Button
              aria-label={t("search.mails-change-name", { name })}
              onPress={() => {
                setChanging(true);
              }}
            >
              {t("search.mails-change")}
            </Button>
            <SearchMailWindow
              search={{ id: mail.search, name: mail.name }}
              isOpen={changing}
              onClose={() => {
                setChanging(false);
              }}
            />
          </>
        )}
        <Button
          aria-label={t("search.mails-stop-name", { name })}
          onPress={() => {
            onStop();
          }}
        >
          {t("search.mails-stop")}
        </Button>
      </span>
    </li>
  );
}

/** That an e-mailed search was stopped, with *Undo*, which sets it again as it was. */
function Stopped({ mail }: { readonly mail: SearchMail }) {
  const again = useSetSearchMail();
  if (again.isSuccess) {
    return null;
  }
  return (
    <Alert tone={alertTone.info}>
      <span>{t("search.mails-stopped", { name: mail.name ?? t("search.mails-lost-search") })}</span>
      <Button
        look={buttonLook.quiet}
        isPending={again.isPending}
        onPress={() => {
          again.mutate({ search: mail.search, choice: mail });
        }}
      >
        {t("common.undo")}
      </Button>
    </Alert>
  );
}

/**
 * *E-mailed searches* (QRY-07, UX design §4.13), at the end of *Notification settings*: each saved
 * search the person gets by e-mail, with its days and hour, *Change* and *Stop* with *Undo*. One that
 * waits for the person says why in its line.
 */
export function SearchMailsSection() {
  const list = useSearchMails();
  const stop = useStopSearchMail();
  const [stopped, setStopped] = useState<SearchMail | null>(null);
  const mails: readonly SearchMail[] = list.data?.search_mails ?? [];
  const heading = t("search.mails-count", { count: mails.length, most: list.data?.most ?? 0 });
  return (
    <Card heading={heading}>
      {list.isError ? <Alert tone={alertTone.danger}>{messageOf(list.error)}</Alert> : null}
      {list.data?.sending === false ? (
        <Alert tone={alertTone.warning}>{t("search.mail-not-sending")}</Alert>
      ) : null}
      {mails.length === 0 && list.isSuccess ? (
        <p {...partOf(part.muted)}>{t("search.mails-none")}</p>
      ) : (
        <ul {...partOf(part.list, { variants: ["snug"] })} aria-label={t("search.mails-title")}>
          {mails.map((mail) => (
            <MailRow
              key={mail.search}
              mail={mail}
              onStop={() => {
                stop.mutate(mail.search, {
                  onSuccess: () => {
                    setStopped(mail);
                  },
                });
              }}
            />
          ))}
        </ul>
      )}
      {stopped === null ? null : <Stopped key={stopped.search} mail={stopped} />}
      {stop.isError ? <Alert tone={alertTone.danger}>{messageOf(stop.error)}</Alert> : null}
    </Card>
  );
}
