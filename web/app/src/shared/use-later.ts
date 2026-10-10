import { useEffect, useState } from "react";

/** Something to do after a pause that starts again with every call, and that stops when the screen goes away. */
export interface Later {
  /** Does `job` after `milliseconds`; a job that was waiting is dropped. */
  readonly schedule: (milliseconds: number, job: () => void) => void;
  /** Drops the job that is waiting. */
  readonly cancel: () => void;
}

/** Waiting for the last of many calls (a key, then another key) before doing something once. */
export function useLater(): Later {
  const [later] = useState(() => {
    let timer: ReturnType<typeof setTimeout> | null = null;
    const cancel = (): void => {
      if (timer !== null) {
        clearTimeout(timer);
        timer = null;
      }
    };
    return {
      cancel,
      schedule: (milliseconds: number, job: () => void): void => {
        cancel();
        timer = setTimeout(job, milliseconds);
      },
    };
  });
  useEffect(() => later.cancel, [later]);
  return later;
}
