import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  Card,
  cardLook,
  LinkRow,
  partOf,
  part,
} from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { messageOf } from "../../shared/errors.ts";
import { paths } from "../../shared/paths.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { QueryGate } from "../shell/index.ts";
import { KeepRulesTables, keepRulesOptions } from "../settings/index.ts";
import { FileState } from "./FileState.tsx";
import { isWaiting, useAskForMyData, useMyExport } from "./my-data.ts";

/**
 * *Download my data* (PRIV-03, ADR-0044, decision 2): one button; while the file is made the button says so and the
 * rest stands still (a state in place, UX §6.4); once it is ready the page offers the link, as the notification does,
 * for as long as the file is kept.
 */
function Download() {
  const found = useMyExport();
  const ask = useAskForMyData();
  const item = found.data?.export ?? null;
  const making = ask.isPending || isWaiting(item);
  return (
    <Card heading={t("my-data.download")} intro={t("my-data.download-intro")}>
      <div {...partOf(part.row)}>
        <Button
          look={buttonLook.primary}
          isPending={making}
          onPress={() => {
            ask.mutate();
          }}
        >
          {making ? t("my-data.making") : t("my-data.download")}
        </Button>
      </div>
      {ask.isError ? <Alert tone={alertTone.danger}>{messageOf(ask.error)}</Alert> : null}
      {found.error === null ? null : (
        <Alert tone={alertTone.danger}>{messageOf(found.error)}</Alert>
      )}
      {found.data === undefined ? null : <FileState item={item} />}
    </Card>
  );
}

/** *Where to correct it*: each kind of the person's data is corrected where it is kept (PRIV-06). */
function WhereToCorrect() {
  return (
    <Card heading={t("my-data.correct-title")} intro={t("my-data.correct-intro")}>
      <ul {...partOf(part.list, { variants: ["snug"] })} aria-label={t("my-data.correct-title")}>
        <LinkRow href={paths.profile}>
          {t("my-data.correct-profile", { place: t("nav.profile") })}
        </LinkRow>
        <LinkRow href={paths.workTimes}>
          {t("my-data.correct-work-times", { place: t("nav.work-times") })}
        </LinkRow>
        <LinkRow href={paths.time}>
          {t("my-data.correct-time", { place: t("nav.timesheet") })}
        </LinkRow>
        <LinkRow href={paths.mySettings}>
          {t("my-data.correct-settings", { place: t("nav.my-settings") })}
        </LinkRow>
      </ul>
      <p {...partOf(part.muted)}>{t("my-data.correct-others")}</p>
    </Card>
  );
}

/**
 * *My settings › My data* (PRIV-03, UX §4.13): the file of everything Bugaboo stores about the person, what is kept and
 * for how long, from the keep rules the server reads (a person reads them, an administrator changes them), and where
 * each kind is corrected.
 */
export function MyDataPage() {
  usePageTitle(t("nav.my-data"));
  const rules = useQuery(keepRulesOptions());
  return (
    <>
      <Card
        heading={t("nav.my-data")}
        level={1}
        look={cardLook.pageHead}
        intro={t("my-data.intro")}
      />
      <Download />
      <QueryGate error={rules.error} data={rules.data?.rules} loading={null}>
        {(found) => <KeepRulesTables rules={found} />}
      </QueryGate>
      <WhereToCorrect />
    </>
  );
}
