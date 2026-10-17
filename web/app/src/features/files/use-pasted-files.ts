import { takePastedFiles } from "@bugaboo/ui";
import { useEffect } from "react";

/**
 * Calls `onFiles` with the files that are pasted anywhere on the page, for as long as the caller
 * is shown and `onFiles` is given: a screenshot pasted from the clipboard comes as a file. Text
 * that is pasted is left alone, and so is a paste that a comment box took already (it says so with
 * `preventDefault`): a screenshot pasted into a comment box goes to that comment.
 */
export function usePastedFiles(onFiles: ((files: readonly File[]) => void) | undefined): void {
  useEffect(() => {
    const listener = (event: ClipboardEvent): void => {
      const files = onFiles === undefined ? [] : takePastedFiles(event);
      if (onFiles !== undefined && files.length > 0) {
        onFiles(files);
      }
    };
    document.addEventListener("paste", listener);
    return () => {
      document.removeEventListener("paste", listener);
    };
  }, [onFiles]);
}
