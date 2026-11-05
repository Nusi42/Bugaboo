import { t } from "@bugaboo/messages";
import { Form, SubmitButton, TextField, partOf, part } from "@bugaboo/ui";

import { type SettingBoxState } from "./use-setting-box.ts";

export interface SettingBoxProps extends SettingBoxState {
  /** Whether what is typed is a length, which stands in the code face. */
  readonly isLength?: boolean;
  /** What the number counts, in words, beside the box (*years*); the box holds the number only. */
  readonly unit?: string | undefined;
}

/**
 * The box of a setting that is typed in, with its hint, the place the value comes from and its own
 * Save button: the one frame of a text, a number, a length and a number with a unit.
 */
export function SettingBox({
  label,
  description,
  problem,
  draft,
  onDraft,
  source,
  isLength = false,
  unit,
  onSubmit,
}: SettingBoxProps) {
  const field = (
    <TextField
      label={label}
      {...(description === undefined ? {} : { description })}
      {...(unit === undefined ? {} : { unit })}
      errorMessage={problem}
      value={draft}
      onChange={onDraft}
    />
  );
  return (
    <Form onSubmit={onSubmit}>
      {isLength ? <span {...partOf(part.lengthBox)}>{field}</span> : field}
      {source === undefined ? null : <p {...partOf(part.settingsSource)}>{source}</p>}
      <div {...partOf(part.row)}>
        <SubmitButton>{t("common.save")}</SubmitButton>
      </div>
    </Form>
  );
}
