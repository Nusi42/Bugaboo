import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, Window, partOf, part } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { useWindowProps } from "../../shared/use-window-props.ts";
import { windowForms } from "../../shared/window-forms.ts";
import { removalPlanOptions, useRemove } from "./person-state.ts";
import { type CommentsChoiceValue, PlanLists, RemovalAnswers } from "./RemovalPlanLists.tsx";

export interface RemovePersonWindowProps {
  readonly id: string;
  readonly name: string;
  readonly standing: Schema<"Standing">;
  /** *Switch off first*: the same switch as on the page. */
  readonly onSwitchOff: () => void;
  readonly switching: boolean;
  /** Whether the viewer holds *Manage people*, which switches a person off. */
  readonly mayManage: boolean;
  /** The refusal of the switch, shown in the window, where the person pressed it. */
  readonly switchError: unknown;
  readonly onClose: () => void;
}

/**
 * Only a switched-off person can be removed: the window says so and offers *Switch off first* to a holder of *Manage
 * people*; to anybody else it says who can, since the server would refuse the switch.
 */
function SwitchOffFirst({
  name,
  onSwitchOff,
  switching,
  mayManage,
  switchError,
}: Pick<
  RemovePersonWindowProps,
  "name" | "onSwitchOff" | "switching" | "mayManage" | "switchError"
>) {
  if (!mayManage) {
    return <Alert tone={alertTone.warning}>{t("removal.switch-off-by-others", { name })}</Alert>;
  }
  return (
    <div {...partOf(part.stack)}>
      <Alert tone={alertTone.warning}>{t("removal.switch-off-first", { name })}</Alert>
      <div {...partOf(part.row)}>
        <Button isPending={switching} onPress={onSwitchOff}>
          {t("removal.switch-off-button")}
        </Button>
      </div>
      {switchError === null ? null : (
        <Alert tone={alertTone.danger}>{messageOf(switchError)}</Alert>
      )}
    </div>
  );
}

interface RemoveFooterProps {
  readonly name: string;
  /** Switched off, the plan read and the password typed. */
  readonly ready: boolean;
  readonly isPending: boolean;
  readonly onClose: () => void;
  readonly onRemove: () => void;
}

/** *Cancel*, and *Remove Malik Weber*, which waits until everything it needs is there. */
function RemoveFooter({ name, ready, isPending, onClose, onRemove }: RemoveFooterProps) {
  return (
    <div {...partOf(part.windowFooterActions)}>
      <Button look={buttonLook.quiet} onPress={onClose}>
        {t("common.cancel")}
      </Button>
      <Button look={buttonLook.danger} isDisabled={!ready} isPending={isPending} onPress={onRemove}>
        {t("removal.confirm", { name })}
      </Button>
    </div>
  );
}

/**
 * The window of *Remove a person* (PRIV-04, UX §5.7, ADR-0044, decision 6): what is deleted now, what stays shown as
 * *Former user*, what the law keeps and until when, all read from the server's plan; the choice about the comments; and
 * *Remove Malik Weber*, which waits until the person is switched off. A removal is final, and the window says so first.
 */
export function RemovePersonWindow(props: RemovePersonWindowProps) {
  const { id, name, standing, onClose } = props;
  const windowProps = useWindowProps(windowForms.removePerson);
  const plan = useQuery(removalPlanOptions(id));
  const [chosen, setChosen] = useState<CommentsChoiceValue>();
  const [password, setPassword] = useState("");
  const remove = useRemove(id);
  const comments = chosen ?? plan.data?.comments ?? "keep";
  const switchedOff = standing === "switched-off";
  const problem = plan.error ?? remove.error;
  return (
    <Window
      {...windowProps}
      title={t("removal.title", { name })}
      isOpen
      ready={plan.data !== undefined || plan.error !== null}
      onOpenChange={(isOpen) => {
        if (!isOpen) {
          onClose();
        }
      }}
      footer={
        <RemoveFooter
          name={name}
          ready={switchedOff && plan.data !== undefined && password !== ""}
          isPending={remove.isPending}
          onClose={onClose}
          onRemove={() => {
            remove.mutate({ comments, password }, { onSuccess: onClose });
          }}
        />
      }
    >
      <div {...partOf(part.stack)}>
        <p {...partOf(part.text)}>{t("removal.intro")}</p>
        {problem === null ? null : <Alert tone={alertTone.danger}>{messageOf(problem)}</Alert>}
        {switchedOff ? null : <SwitchOffFirst {...props} />}
        {plan.data === undefined ? null : <PlanLists plan={plan.data} />}
        <RemovalAnswers
          name={name}
          comments={comments}
          onComments={setChosen}
          password={password}
          onPassword={setPassword}
        />
      </div>
    </Window>
  );
}
