import { t } from "@bugaboo/messages";
import { Alert, ConfirmDialog } from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { MovePreviewView } from "./MovePreviewView.tsx";
import { ProjectPicker } from "./ProjectPicker.tsx";
import styles from "./Projects.module.css";
import { useMovePreview, useMoveProject, type OneProject } from "./use-projects.ts";

/** What the preview of the picked place is: being worked out, refused, or what would change. */
function PreviewOf({ project, to }: { readonly project: string; readonly to: string | null }) {
  const preview = useMovePreview(project, to, true);
  if (preview.isError) {
    return <Alert danger>{messageOf(preview.error)}</Alert>;
  }
  if (preview.isPending) {
    return <p className={styles.muted}>{t("projects.preview-loading")}</p>;
  }
  return <MovePreviewView preview={preview.data} />;
}

/** What the last move did: it failed with a sentence, or the project was moved. */
function Outcome(props: {
  readonly failed: string | null;
  readonly moved: boolean;
  readonly name: string;
}) {
  const { failed, moved, name } = props;
  if (failed !== null) {
    return <Alert danger>{failed}</Alert>;
  }
  return moved ? <Alert success>{t("projects.moved", { name })}</Alert> : null;
}

/**
 * *Move*: pick where the project goes, read in plain words who gains or loses access, and then
 * confirm (PRJ-03, *Show, then do*). The move asks for the version of the preview that was read; a
 * tree that changed meanwhile asks to look again.
 */
export function MoveProject({ project }: { readonly project: OneProject }) {
  const { id, name } = project.project;
  // Nothing is picked until the person picks; the top level is a pick (`to: null`).
  const [picked, setPicked] = useState<{ readonly to: string | null } | null>(null);
  const [moved, setMoved] = useState(false);
  const to = picked === null ? null : picked.to;
  const preview = useMovePreview(id, to, picked !== null);
  const move = useMoveProject();
  const ready = picked !== null && preview.isSuccess && !preview.data.blocked;
  return (
    <div className={styles.stack}>
      <p className={styles.text}>{t("projects.move-text", { name })}</p>
      <Outcome failed={move.isError ? messageOf(move.error) : null} moved={moved} name={name} />
      <div className={styles.row}>
        <ConfirmDialog
          triggerLabel={t("projects.move-button")}
          title={t("projects.move-dialog-title", { name })}
          text={t("projects.move-dialog-text")}
          confirmLabel={t("projects.move-confirm")}
          cancelLabel={t("common.cancel")}
          confirmDisabled={!ready}
          onClosed={() => {
            setPicked(null);
          }}
          onConfirm={() => {
            setMoved(false);
            move.mutate(
              { id, to, preview: preview.data?.preview ?? "" },
              {
                onSuccess: () => {
                  setMoved(true);
                },
              },
            );
          }}
        >
          <ProjectPicker
            moving={id}
            top={project.may_move_to_top}
            value={picked?.to}
            onChange={(place) => {
              setPicked({ to: place });
            }}
          />
          {picked === null ? null : <PreviewOf project={id} to={picked.to} />}
        </ConfirmDialog>
      </div>
    </div>
  );
}
