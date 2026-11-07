import type { Schema } from "@bugaboo/api-client";
import { t, type MessageKey } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  ChoiceGroup,
  Form,
  SubmitButton,
  partOf,
  part,
} from "@bugaboo/ui";
import { useState } from "react";

import { toggled } from "../../shared/toggled.ts";
import { choiceName, ProjectLimitPicker } from "../my-settings/index.ts";
import type { ConnectRequest } from "./request.ts";
import { useDecision } from "./use-decision.ts";

export interface ConnectFormProps {
  readonly request: ConnectRequest;
  /** What the app asked for. The person may allow less, never more. */
  readonly asked: Schema<"KeyChoice">;
  /** The web address the app comes back to; none when it comes back to this computer. */
  readonly webAddress: string | null;
  /** The name the app gave itself; on a phone the heading of the card is not shown. */
  readonly app: string;
}

/** What an app can be given: the two plain choices of an access key. */
type Scope = "see-only" | "see-and-change";

/** The sentence that says what each choice means for an app. */
const helps: Readonly<Record<Scope, MessageKey>> = {
  "see-only": "connect.help.see-only",
  "see-and-change": "connect.help.see-and-change",
};

/**
 * The scopes the form offers, what the app asked first and *See only* below it. An app that asked
 * to see only is not offered more (ADR-0042, the consent step).
 */
function offered(asked: Schema<"KeyChoice">): readonly Scope[] {
  return asked === "see-and-change" ? ["see-and-change", "see-only"] : ["see-only"];
}

interface MayProps {
  readonly choices: readonly Scope[];
  readonly scope: Scope;
  readonly onScope: (scope: Scope) => void;
}

/** *What it may do*: both choices, each with what it means, or one plain statement when only one is offered. */
function May({ choices, scope, onScope }: MayProps) {
  if (choices.length === 1) {
    return <p {...partOf(part.text)}>{t("connect.may-only")}</p>;
  }
  return (
    <>
      <ChoiceGroup
        legend={t("access-keys.may")}
        choices={choices.map((value) => ({ value, label: t(choiceName(value)) }))}
        value={scope}
        onChange={onScope}
      />
      {choices.map((value) => (
        <p key={value} {...partOf(part.muted)}>
          {t("connect.choice-help", { choice: t(choiceName(value)), what: t(helps[value]) })}
        </p>
      ))}
    </>
  );
}

/** *Allow* and *Cancel*, with what the app may do and in which projects. */
export function ConnectForm({ request, asked, webAddress, app }: ConnectFormProps) {
  const choices = offered(asked);
  const [scope, setScope] = useState<Scope>(choices[0] ?? "see-only");
  const [picked, setPicked] = useState<readonly string[]>([]);
  const deciding = useDecision(request);
  return (
    <Form
      onSubmit={() => {
        deciding.send({ allow: true, scope, projects: [...picked] });
      }}
    >
      <p {...partOf(part.text)}>{t("connect.acts-as", { app })}</p>
      <May choices={choices} scope={scope} onScope={setScope} />
      <ProjectLimitPicker
        picked={picked}
        onPick={(project, on) => {
          setPicked((now) => toggled(now, project, on));
        }}
      />
      <p {...partOf(part.muted)}>
        {webAddress === null
          ? t("connect.where-computer")
          : t("connect.where-web", { address: webAddress })}
      </p>
      <p {...partOf(part.muted)}>{t("connect.name-unchecked")}</p>
      <p {...partOf(part.muted)}>{t("connect.take-back-note")}</p>
      {deciding.problem === null ? null : <Alert tone={alertTone.danger}>{deciding.problem}</Alert>}
      {deciding.leaving ? (
        <Alert tone={alertTone.success}>
          {t("connect.going-back")} {t("connect.going-back-help")}
        </Alert>
      ) : null}
      <div {...partOf(part.row, { variants: ["stretch"] })}>
        <SubmitButton isPending={deciding.isPending}>{t("connect.allow")}</SubmitButton>
        <Button
          onPress={() => {
            deciding.send({ allow: false, scope, projects: [] });
          }}
          isDisabled={deciding.isPending || deciding.leaving}
        >
          {t("connect.cancel")}
        </Button>
      </div>
    </Form>
  );
}
