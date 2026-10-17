import { useEffect } from "react";

/**
 * Calls `onFiles` with the files that are pasted anywhere on the page, for as long as the caller
 * is shown and `onFiles` is given: a screenshot pasted from the clipboard comes as a file. Text
 * that is pasted is left alone.
 */
export function usePastedFiles(onFiles: ((files: readonly File[]) => void) | undefined): void {
  useEffect(() => {
    const listener = (event: ClipboardEvent): void => {
      const files = Array.from(event.clipboardData?.files ?? []);
      if (onFiles !== undefined && files.length > 0) {
        event.preventDefault();
        onFiles(files);
      }
    };
    document.addEventListener("paste", listener);
    return () => {
      document.removeEventListener("paste", listener);
    };
  }, [onFiles]);
}
