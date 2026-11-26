import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, TextField } from "@bugaboo/ui";

import { DayBox } from "../../shared/DayBox.tsx";
import { FormWindow } from "../../shared/FormWindow.tsx";
import { windowForms } from "../../shared/window-forms.ts";
import { stateText } from "./release-words.ts";
import type { Release } from "./releases.ts";
import { useReleaseForm, type ReleaseForm } from "./use-release-form.ts";

export interface ReleaseWindowProps {
  /** The project the release is made in, or already was. */
  readonly project: { readonly id: string; readonly name: string };
  /** The release to change; none makes a new one. */
  readonly release?: Release | undefined;
  /** How many issues are planned for it, and how many found in it, as the person sees them. */
  readonly counts?: { readonly planned: number; readonly found: number } | undefined;
  readonly onClose: () => void;
}

/** The way out of a release that is not a save: *Delete*, the way on in its state, and *Retire the release*. */
function Ways({ release, form }: { readonly release: Release; readonly form: ReleaseForm }) {
  return (
    <>
      <Button look={buttonLook.quietDanger} isDisabled={form.busy} onPress={form.remove}>
        {t("releases.delete")}
      </Button>
      {release.state === "planned" ? (
        <Button
          isDisabled={form.busy}
          onPress={() => {
            form.withState("released");
          }}
        >
          {t("releases.mark-released")}
        </Button>
      ) : (
        <Button
          isDisabled={form.busy}
          onPress={() => {
            form.withState("planned");
          }}
        >
          {t("releases.back-to-planned")}
        </Button>
      )}
      {release.state === "retired" ? null : (
        <Button
          isDisabled={form.busy}
          onPress={() => {
            form.withState("retired");
          }}
        >
          {t("releases.retire")}
        </Button>
      )}
    </>
  );
}

/** What is said of a release being changed: its state, the issues that name it, and why it may go back. */
function State({
  release,
  counts,
  form,
}: Pick<ReleaseWindowProps, "counts"> & { readonly release: Release; readonly form: ReleaseForm }) {
  return (
    <>
      <p>
        {t("releases.state-line", {
          state: stateText(release.state),
          planned: counts?.planned ?? 0,
          found: counts?.found ?? 0,
        })}
      </p>
      {release.state === "planned" ? null : (
        <TextField
          label={t("releases.reason")}
          description={t("releases.reason-help")}
          value={form.reason}
          onChange={form.setReason}
        />
      )}
      <p>{t("releases.retired-note")}</p>
    </>
  );
}

/**
 * The window of a release (UX §4.16, ISS-08): its name and *Release date*, its state with how many
 * issues are planned for it and found in it, and the ways to go on: *Save the release*, *Mark as
 * released*, *Retire the release* and *Delete*. The server judges every change; a delete that issues
 * name is refused with the sentence that says to retire it instead.
 */
export function ReleaseWindow({ project, release, counts, onClose }: ReleaseWindowProps) {
  const form = useReleaseForm({ project, release, onClose });
  return (
    <FormWindow
      form={windowForms.release}
      title={
        release === undefined
          ? t("releases.create-title", { project: project.name })
          : t("releases.window-title", { name: release.name, project: release.project.name })
      }
      submit={release === undefined ? t("releases.create-it") : t("releases.save")}
      onSubmit={form.save}
      onClose={onClose}
      mistakes={form.mistakes.list}
      extra={release === undefined ? null : <Ways release={release} form={form} />}
    >
      <TextField label={t("releases.name")} value={form.name} onChange={form.setName} />
      <DayBox
        label={t("releases.release-date")}
        value={form.date}
        onChange={form.setDate}
        onMistake={form.mistakes.mark("date")}
      />
      {release === undefined ? null : <State release={release} counts={counts} form={form} />}
      {form.problem === undefined ? null : <Alert tone={alertTone.danger}>{form.problem}</Alert>}
    </FormWindow>
  );
}
