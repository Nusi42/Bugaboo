/** What a paste event gives: the clipboard, and whether somebody took the paste already. */
export interface Paste {
  readonly clipboardData: DataTransfer | null;
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
