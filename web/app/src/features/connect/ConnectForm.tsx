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
  useLayout,
} from "@bugaboo/ui";
import { useState } from "react";

import { toggled } from "../../shared/toggled.ts";
import { choiceName } from "../my-settings/index.ts";
import { ProjectsReach, type Reach } from "./ProjectsReach.tsx";
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

export interface ButtonsProps {
  readonly phone: boolean;
  readonly isPending: boolean;
  /** The answer is on its way back to the app. */
  readonly leaving: boolean;
  /** Only some projects is chosen and none is ticked yet. */
  readonly waitsForProjects: boolean;
  readonly onCancel: () => void;
}

/**
 * *Cancel* at the start and the main button at the end; on a phone the main button fills the width
 * and *Cancel* stands under it (UX §6).
 */
function Buttons({ phone, isPending, leaving, waitsForProjects, onCancel }: ButtonsProps) {
  const allow = (
    <SubmitButton wide={phone} isPending={isPending} isDisabled={waitsForProjects}>
      {t("connect.allow")}
    </SubmitButton>
  );
  const cancel = (
    <Button wide={phone} onPress={onCancel} isDisabled={isPending || leaving}>
      {t("connect.cancel")}
    </Button>
  );
  return phone ? (
    <div {...partOf(part.stack)}>
      {allow}
      {cancel}
    </div>
  ) : (
    <div {...partOf(part.row)}>
      {cancel}
      <span {...partOf(part.rowEnd)}>{allow}</span>
    </div>
  );
}

/** *Allow* and *Cancel*, with what the app may do and in which projects. */
export function ConnectForm({ request, asked, webAddress, app }: ConnectFormProps) {
  const choices = offered(asked);
  const [scope, setScope] = useState<Scope>(choices[0] ?? "see-only");
  const [picked, setPicked] = useState<readonly string[]>([]);
  const [reach, setReach] = useState<Reach>("all");
  const phone = useLayout() === "phone";
  const deciding = useDecision(request);
  return (
    <Form
      onSubmit={() => {
        deciding.send({ allow: true, scope, projects: reach === "some" ? [...picked] : [] });
      }}
    >
      {phone ? <h2 {...partOf(part.cardHeading)}>{t("connect.title", { app })}</h2> : null}
      <p {...partOf(part.text)}>{t("connect.acts-as", { app })}</p>
      <May choices={choices} scope={scope} onScope={setScope} />
      <ProjectsReach
        reach={reach}
        onReach={setReach}
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
      <Buttons
        phone={phone}
        isPending={deciding.isPending}
        leaving={deciding.leaving}
        waitsForProjects={reach === "some" && picked.length === 0}
        onCancel={() => {
          deciding.send({ allow: false, scope, projects: [] });
        }}
      />
    </Form>
  );
}
