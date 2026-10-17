import { FileTrigger } from "react-aria-components";

import { Button } from "./Button.tsx";

export interface FilePickerProps {
  /** The text of the button that opens the choice of files of the device. */
  readonly chooseLabel: string;
  /** The name of that button for screen readers when the text alone is not enough. */
  readonly chooseName?: string;
  /** Gets the files that were chosen. */
  readonly onFiles: (files: readonly File[]) => void;
}

/** A button that opens the choice of files of the device; the keyboard and a screen reader use it. */
export function FilePicker({ chooseLabel, chooseName, onFiles }: FilePickerProps) {
  return (
    <FileTrigger
      allowsMultiple
      onSelect={(chosen) => {
        if (chosen !== null) {
          onFiles(Array.from(chosen));
        }
      }}
    >
      <Button {...(chooseName === undefined ? {} : { "aria-label": chooseName })}>
        {chooseLabel}
      </Button>
    </FileTrigger>
  );
}
