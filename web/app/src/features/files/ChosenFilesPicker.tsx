import { t } from "@bugaboo/messages";
import { Button, buttonLook, FilePicker, partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { chosenKey, type ChosenFiles } from "./chosen-files.ts";
import { formatSize } from "./size.ts";
import { FileName } from "./FileName.tsx";

/** *Add files* under the box of a comment, and the list of what was chosen, each with *Take out*. */
export function ChosenFilesPicker({
  chosen,
  end,
}: {
  readonly chosen: ChosenFiles;
  /** What stands at the end of the row of the button that adds files (the button that sends the comment). */
  readonly end?: ReactNode;
}) {
  return (
    <div {...partOf(part.stack)}>
      <div {...partOf(part.row)}>
        <FilePicker chooseLabel={t("files.add-to-comment")} onFiles={chosen.add} />
        {end === undefined ? null : <span {...partOf(part.rowEnd)}>{end}</span>}
      </div>
      {chosen.files.length === 0 ? null : (
        <ul {...partOf(part.list, { variants: ["tight"] })} aria-label={t("files.for-comment")}>
          {chosen.files.map((file, at) => (
            <li key={chosenKey(file, at)}>
              <div {...partOf(part.row)}>
                <FileName name={file.name} />
                <span {...partOf(part.filesMeta)}>{formatSize(file.size)}</span>
                <Button
                  look={buttonLook.quiet}
                  aria-label={t("files.take-out", { name: file.name })}
                  onPress={() => {
                    chosen.takeOut(file);
                  }}
                >
                  {t("files.take-out-short")}
                </Button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
