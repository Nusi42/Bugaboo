import type { ReactNode } from "react";
import { DropZone } from "react-aria-components";

import { FilePicker } from "./FilePicker.tsx";
import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

export interface FileDropProps {
  /** What the place says, next to its button: where to drop, what to do instead. */
  readonly children: ReactNode;
  /** The name of the place for screen readers. */
  readonly label: string;
  /** The text of the button that opens the choice of files of the device (*Upload*). */
  readonly chooseLabel: string;
  /** Gets the files that were dropped on the place or chosen with the button. */
  readonly onFiles: (files: readonly File[]) => void;
}

/**
 * A place that takes files: dropped on it, or chosen with its button (the keyboard and a screen
 * reader use the button). React Aria marks the place while files are over it, and the theme
 * draws that (`files.drop`). Folders and text that are dropped are left out.
 */
export function FileDrop({ children, label, chooseLabel, onFiles }: FileDropProps) {
  return (
    <DropZone
      aria-label={label}
      {...partOf(part.filesDrop)}
      onDrop={(event) => {
        const files = event.items.flatMap((item) => (item.kind === "file" ? [item.getFile()] : []));
        void Promise.all(files).then(onFiles);
      }}
    >
      <FilePicker chooseLabel={chooseLabel} onFiles={onFiles} />
      {children}
    </DropZone>
  );
}
