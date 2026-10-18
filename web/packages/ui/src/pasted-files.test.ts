import { describe, expect, it, vi } from "vitest";

import { passPastedFiles, type Paste } from "./pasted-files.ts";

/** A paste with these files; `taken` says whether somebody took it already. */
function pasteOf(files: readonly File[], taken = false): Paste & { readonly took: () => boolean } {
  let prevented = taken;
  return {
    clipboardData: { files },
    get defaultPrevented() {
      return prevented;
    },
    preventDefault: () => {
      prevented = true;
    },
    took: () => prevented,
  };
}

describe("att_01 pasting files", () => {
  const screenshot = new File(["x"], "screenshot.png", { type: "image/png" });

  it("hands the files over and takes the paste", () => {
    const onFiles = vi.fn<(files: readonly File[]) => void>();
    const paste = pasteOf([screenshot]);
    passPastedFiles(paste, onFiles);
    expect(onFiles).toHaveBeenCalledWith([screenshot]);
    expect(paste.took()).toBe(true);
  });

  it("leaves a paste alone that has no files, was taken, or has nobody to take it", () => {
    const onFiles = vi.fn<(files: readonly File[]) => void>();
    passPastedFiles(pasteOf([]), onFiles);
    passPastedFiles(pasteOf([screenshot], true), onFiles);
    const unclaimed = pasteOf([screenshot]);
    passPastedFiles(unclaimed);
    expect(onFiles).not.toHaveBeenCalled();
    expect(unclaimed.took()).toBe(false);
  });
});
