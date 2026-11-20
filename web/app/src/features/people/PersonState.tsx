import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  Card,
  ConfirmDialog,
  partOf,
  part,
} from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { usePermissions } from "../../shared/use-me.ts";
import { useSwitch } from "./person-state.ts";
import { RemovePersonWindow } from "./RemovePersonWindow.tsx";

export interface PersonStateProps {
  readonly id: string;
  readonly name: string;
  readonly standing: Schema<"Standing">;
}

/** *Switch off*, after a short question that names what stops; *Switch on* gives the sign-in back at once. */
function SwitchButton({
  name,
  standing,
  onOff,
  onOn,
  busy,
}: Omit<PersonStateProps, "id"> & {
  readonly onOff: () => void;
  readonly onOn: () => void;
  readonly busy: boolean;
}) {
  if (standing === "switched-off") {
    return (
      <Button isPending={busy} onPress={onOn}>
        {t("person.switch-on")}
      </Button>
    );
  }
  return (
    <ConfirmDialog
      triggerLabel={t("person.switch-off")}
      title={t("person.switch-off-title", { name })}
      text={t("person.switch-off-text", { name })}
      confirmLabel={t("person.switch-off")}
      cancelLabel={t("common.cancel")}
      confirmLook={buttonLook.danger}
      confirmDisabled={busy}
      onConfirm={onOff}
    />
  );
}

/** Whether the person may sign in, in a sentence. */
function standingText(name: string, standing: Schema<"Standing">): string {
  return standing === "switched-off"
    ? t("person.switched-off", { name })
    : t("person.active", { name });
}

/**
 * *Sign-in and removal* on a person's page (PRIV-04, ADR-0044, decisions 5 and 6): whether the person may sign in,
 * *Switch off* or *Switch on* for a holder of *Manage people*, and *Remove this person…* for a holder of *Remove
 * people*, which opens the window that says what goes and what stays. It stands last on the page, so the sentence that
 * changes with a switch moves nothing under it (UX §6.4).
 */
export function PersonState({ id, name, standing }: PersonStateProps) {
  const held = usePermissions();
  const switching = useSwitch(id);
  const [removing, setRemoving] = useState(false);
  const mayRemove = held.includes("remove-people");
  return (
    <Card heading={t("person.state-title")}>
      <div {...partOf(part.row)}>
        {held.includes("manage-people") ? (
          <SwitchButton
            name={name}
            standing={standing}
            onOff={switching.off}
            onOn={switching.on}
            busy={switching.busy}
          />
        ) : null}
        {mayRemove ? (
          <Button
            look={buttonLook.danger}
            onPress={() => {
              setRemoving(true);
            }}
          >
            {t("person.remove")}
          </Button>
        ) : null}
      </div>
      {switching.error === null ? null : (
        <Alert tone={alertTone.danger}>{messageOf(switching.error)}</Alert>
      )}
      <p {...partOf(part.text)}>{standingText(name, standing)}</p>
      {removing ? (
        <RemovePersonWindow
          id={id}
          name={name}
          standing={standing}
          onSwitchOff={switching.off}
          switching={switching.busy}
          onClose={() => {
            setRemoving(false);
          }}
        />
      ) : null}
    </Card>
  );
}
