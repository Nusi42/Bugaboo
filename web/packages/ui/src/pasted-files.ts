import { useEffect } from "react";

/** What a paste event gives: the clipboard, and whether somebody took the paste already. */
export interface Paste {
  readonly clipboardData: { readonly files: ArrayLike<File> } | null;
  readonly defaultPrevented: boolean;
  readonly preventDefault: () => void;
}

/**
 * Takes the files out of a paste: a screenshot from the clipboard comes as a file. The paste is
 * then taken (`preventDefault`), so no text is written for it and nobody else takes it again. A
 * paste of text, or one that somebody took already, gives no file and is left alone.
 */
export function takePastedFiles(paste: Paste): readonly File[] {
  const files = Array.from(paste.clipboardData?.files ?? []);
  if (files.length === 0 || paste.defaultPrevented) {
    return [];
  }
  paste.preventDefault();
  return files;
}

/**
 * Hands the files of a paste to `onFiles` and takes the paste. Nothing is done without `onFiles`, for
 * a paste of text, or for one that somebody took already. The one rule of a paste that carries
 * files, for the box of a comment and for the whole page.
 */
export function passPastedFiles(paste: Paste, onFiles?: (files: readonly File[]) => void): void {
  const files = onFiles === undefined ? [] : takePastedFiles(paste);
  if (onFiles !== undefined && files.length > 0) {
    onFiles(files);
  }
}

/**
 * Calls `onFiles` with the files that are pasted anywhere on the page, for as long as the caller
 * is shown and `onFiles` is given: a screenshot pasted from the clipboard comes as a file. A paste
 * that a text box took already (it says so with `preventDefault`) is left alone, so a screenshot
 * pasted into a comment box goes to that comment.
 */
export function usePastedFiles(onFiles: ((files: readonly File[]) => void) | undefined): void {
  useEffect(() => {
    const listener = (event: ClipboardEvent): void => {
      passPastedFiles(event, onFiles);
    };
    document.addEventListener("paste", listener);
    return () => {
      document.removeEventListener("paste", listener);
    };
  }, [onFiles]);
}
