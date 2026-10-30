import { notificationKindWords, notificationSettings } from "@bugaboo/api-client";
import { t, type MessageKey } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  Card,
  SaveStatus,
  Switch,
  partOf,
  part,
} from "@bugaboo/ui";
import { useId } from "react";

import { changeMySettings } from "../../shared/change-my-settings.ts";
import { isSettingKey, type Setting } from "../../shared/my-settings.ts";
import { saveLabels } from "../../shared/save-labels.ts";
import { useMySettings, useSaveProgress } from "../../shared/use-my-settings.ts";
import { usePageTitle } from "../../shared/use-page.ts";

type Kind = (typeof notificationKindWords)[number];

/** The words of each kind of event, from the message catalog. */
const kindName = {
  mentioned: "notification.kind.mentioned",
  assigned: "notification.kind.assigned",
  "followed-activity": "notification.kind.followed-activity",
  "followed-fields": "notification.kind.followed-fields",
  "run-ended": "notification.kind.run-ended",
  "edit-notice": "notification.kind.edit-notice",
} as const satisfies Record<Kind, MessageKey>;

/** The two ways of hearing of an event, with the words the column of each has. */
const ways = [
  { way: "app", name: "notifications.in-app" },
  { way: "mail", name: "notifications.by-mail" },
] as const satisfies readonly { readonly way: "app" | "mail"; readonly name: MessageKey }[];

interface WayProps {
  readonly setting: Setting | undefined;
  readonly name: MessageKey;
}

/** One switch: whether the person hears of this kind of event this way. A locked one only says so. */
function Way({ setting, name }: WayProps) {
  if (setting === undefined) {
    return null;
  }
  const on = setting.value === true;
  const key = setting.key;
  if (setting.locked || !isSettingKey(key)) {
    return (
      <span {...partOf(part.muted)}>
        {t("notifications.locked-way", {
          way: t(name),
          value: on ? t("common.on") : t("common.off"),
        })}
      </span>
    );
  }
  return (
    <Switch
      label={t(name)}
      isSelected={on}
      onChange={(next) => {
        void changeMySettings([{ key, value: next }]);
      }}
    />
  );
}

/** The row of one kind of event: its words, and its switch for each way. */
function KindRow({
  kind,
  settings,
}: {
  readonly kind: Kind;
  readonly settings: readonly Setting[];
}) {
  const id = useId();
  const keys = notificationSettings[kind];
  return (
    <li {...partOf(part.customizeRow)}>
      <span id={id} {...partOf(part.customizeRowKind)}>
        {t(kindName[kind])}
      </span>
      <fieldset aria-labelledby={id} {...partOf(part.customizeRowWays)}>
        {ways.map(({ way, name }) => (
          <Way key={way} setting={settings.find((each) => each.key === keys[way])} name={name} />
        ))}
      </fieldset>
    </li>
  );
}

/**
 * *Customize* (UX §4.11, NTF-01): one row for each kind of event, with *In the app* and *By
 * e-mail*. A choice shows at once and is saved in the background; an organization may lock one.
 */
export function CustomizePage() {
  usePageTitle(t("nav.notification-settings"));
  const settings = useMySettings()?.settings ?? [];
  const progress = useSaveProgress();
  const ownChoices = notificationKindWords
    .flatMap((kind) => [notificationSettings[kind].app, notificationSettings[kind].mail])
    .filter((key) => settings.some((each) => each.key === key && each.own && !each.locked));
  return (
    <Card
      heading={t("nav.notification-settings")}
      level={1}
      intro={t("notifications.customize-intro")}
    >
      <SaveStatus state={progress.state} labels={saveLabels()} />
      {progress.problem === null ? null : <Alert tone={alertTone.danger}>{progress.problem}</Alert>}
      <ul {...partOf(part.list)} aria-label={t("notifications.kind")}>
        {notificationKindWords.map((kind) => (
          <KindRow key={kind} kind={kind} settings={settings} />
        ))}
      </ul>
      {ownChoices.length === 0 ? null : (
        <div {...partOf(part.customizeActions)}>
          <Button
            look={buttonLook.quiet}
            onPress={() => {
              void changeMySettings(ownChoices.map((key) => ({ key, value: null })));
            }}
          >
            {t("setting.ui.reset")}
          </Button>
        </div>
      )}
    </Card>
  );
}
