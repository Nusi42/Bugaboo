import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  ChoiceGroup,
  choiceGroupLook,
  Window,
  partOf,
  part,
} from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { useWindowProps } from "../../shared/use-window-props.ts";
import { windowForms } from "../../shared/window-forms.ts";
import { removalPlanOptions, useRemove } from "./person-state.ts";
import { PlanLists } from "./RemovalPlanLists.tsx";

/** What happens to the comments of a removed person (`RemovedTexts` of the API). */
type CommentsChoiceValue = Schema<"RemovedTexts">;

export interface RemovePersonWindowProps {
  readonly id: string;
  readonly name: string;
  readonly standing: Schema<"Standing">;
  /** *Switch off first*: the same switch as on the page. */
  readonly onSwitchOff: () => void;
  readonly switching: boolean;
  readonly onClose: () => void;
}

/** The choice about the person's comments: keep them, shown as *Former user*, or delete them (ADR-0044, decision 7). */
function CommentsChoice({
  name,
  value,
  onChange,
}: {
  readonly name: string;
  readonly value: CommentsChoiceValue;
  readonly onChange: (value: CommentsChoiceValue) => void;
}) {
  return (
    <ChoiceGroup
      legend={t("removal.comments", { name })}
      look={choiceGroupLook.radios}
      choices={[
        { value: "keep", label: t("setting.privacy.removed-texts.keep") },
        { value: "delete", label: t("setting.privacy.removed-texts.delete") },
      ]}
      value={value}
      onChange={onChange}
    />
  );
}

/** Only a switched-off person can be removed: the window says so and offers *Switch off first*. */
function SwitchOffFirst({
  name,
  onSwitchOff,
  switching,
}: Pick<RemovePersonWindowProps, "name" | "onSwitchOff" | "switching">) {
  return (
    <div {...partOf(part.stack)}>
      <Alert tone={alertTone.warning}>{t("removal.switch-off-first", { name })}</Alert>
      <div {...partOf(part.row)}>
        <Button isPending={switching} onPress={onSwitchOff}>
          {t("removal.switch-off-button")}
        </Button>
      </div>
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
  const remove = useRemove(id);
  const comments = chosen ?? plan.data?.comments ?? "keep";
  const switchedOff = standing === "switched-off";
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
        <div {...partOf(part.windowFooterActions)}>
          <Button look={buttonLook.quiet} onPress={onClose}>
            {t("common.cancel")}
          </Button>
          <Button
            look={buttonLook.danger}
            isDisabled={!switchedOff || plan.data === undefined}
            isPending={remove.isPending}
            onPress={() => {
              remove.mutate(comments, { onSuccess: onClose });
            }}
          >
            {t("removal.confirm", { name })}
          </Button>
        </div>
      }
    >
      <div {...partOf(part.stack)}>
        <p {...partOf(part.text)}>{t("removal.intro")}</p>
        {plan.error === null ? null : (
          <Alert tone={alertTone.danger}>{messageOf(plan.error)}</Alert>
        )}
        {remove.isError ? <Alert tone={alertTone.danger}>{messageOf(remove.error)}</Alert> : null}
        {switchedOff ? null : <SwitchOffFirst {...props} />}
        {plan.data === undefined ? null : <PlanLists plan={plan.data} />}
        <CommentsChoice name={name} value={comments} onChange={setChosen} />
      </div>
    </Window>
  );
}
