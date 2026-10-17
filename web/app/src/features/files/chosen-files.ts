import { useState } from "react";

import { useFileRules } from "./files.ts";
import { transfers } from "./transfers.ts";

/** The files a person chose for a comment they have not posted yet. */
export interface ChosenFiles {
  readonly files: readonly File[];
  readonly add: (files: readonly File[]) => void;
  readonly takeOut: (file: File) => void;
  /** Sends them to the comment that was just posted, and forgets them. */
  readonly sendTo: (comment: string) => void;
}

/**
 * The files for a comment that is being written (UX §4.4): they wait on the device, and go to the
 * comment as soon as it is posted, because a comment has an ID only then. The limit of the
 * project is read once, for the words that refuse a file that is too big.
 */
export function useChosenFiles(issue: string): ChosenFiles {
  const [files, setFiles] = useState<readonly File[]>([]);
  const limit = useFileRules(issue).data?.limit_bytes ?? 0;
  return {
    files,
    add: (more) => {
      setFiles((all) => [...all, ...more]);
    },
    takeOut: (file) => {
      setFiles((all) => all.filter((each) => each !== file));
    },
    sendTo: (comment) => {
      transfers.add({ issue, comment }, files, limit);
      setFiles([]);
    },
  };
}

/** What tells one chosen file from another in a list: the same file may be chosen twice. */
export function chosenKey(file: File, at: number): string {
  return [file.name, file.size, file.lastModified, at].join("-");
}
