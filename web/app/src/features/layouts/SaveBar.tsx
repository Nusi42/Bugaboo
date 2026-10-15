import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  SaveStatus,
  partOf,
  part,
  type SaveState,
} from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { saveLabels } from "../../shared/save-labels.ts";

export interface SaveBarProps {
  /** There are changes that are not saved. */
  readonly dirty: boolean;
  /** The person holds the lock, so a save can be made. */
  readonly canSave: boolean;
  readonly pending: boolean;
  /** The last save worked. */
  readonly saved: boolean;
  /** Why the last save did not work. */
  readonly error: unknown;
  readonly onSave: () => void;
  readonly onDiscard: () => void;
}

/** The state of the last change as the words under the buttons say it. */
function stateOf({ dirty, saved }: Pick<SaveBarProps, "dirty" | "saved">): SaveState {
  if (dirty) {
    return "not-saved";
  }
  return saved ? "saved" : "idle";
}

/** *Save layout* and *Throw my changes away*, with what happened to the last save. */
export function SaveBar(props: SaveBarProps) {
  const { dirty, canSave, pending, error, onSave, onDiscard } = props;
  return (
    <>
      {error === null || error === undefined ? null : (
        <Alert tone={alertTone.danger}>{messageOf(error)}</Alert>
      )}
      <div {...partOf(part.row)}>
        <Button
          look={buttonLook.primary}
          isPending={pending}
          isDisabled={!dirty || !canSave}
          onPress={onSave}
        >
          {t("layouts.save")}
        </Button>
        <Button isDisabled={!dirty} onPress={onDiscard}>
          {t("layouts.draft-drop")}
        </Button>
        <SaveStatus state={stateOf(props)} labels={saveLabels()} />
      </div>
    </>
  );
}
