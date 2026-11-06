/**
 * What is typed in a cell, and what of the week the server read again it may take over. The saved
 * text is taken (`sync`) only when no save of this cell waits, and then when it is the value sent
 * last, or when nothing newer was typed and the cell is not the one the person is in. So a text
 * that was read before the last save came back never replaces what was typed after it, and a
 * refused save leaves the draft where it was. When saves overlap, a text that came while they were
 * on their way may be older than the last of them, so it is never taken when they are done: only a
 * text read after that (or the value sent last) can replace what was typed.
 */
export class CellDraft {
  private latest: string;
  private synced: string;
  private sent: string | null = null;
  private waiting = 0;
  private overlapped = false;
  private focused = false;

  constructor(text: string) {
    this.latest = text;
    this.synced = text;
  }

  /** The person typed `value`. */
  type(value: string): void {
    this.latest = value;
  }

  enter(): void {
    this.focused = true;
  }

  leave(): void {
    this.focused = false;
  }

  /** The value of the box is taken back to the saved text (`Escape`, or nothing was different). */
  giveUp(saved: string): void {
    this.latest = saved;
    this.synced = saved;
  }

  /** A save of `value` is on its way; the function it returns is told how it went. */
  saving(value: string): (saved: boolean) => void {
    this.sent = value;
    this.overlapped ||= this.waiting > 0;
    this.waiting += 1;
    return (saved) => {
      this.waiting -= 1;
      if (saved) {
        // What was sent is saved: only what is typed after it is a draft, so the text the server
        // writes for it (`1.5` is `1:30`) can replace it.
        this.synced = value;
      }
    };
  }

  /**
   * Takes the `text` read while saves were on its way, once they are done; says whether it did.
   * It is dropped when the saves overlapped, as it may be older than the last one.
   */
  syncHeld(text: string): boolean {
    if (this.waiting > 0) {
      return false;
    }
    const overlapped = this.overlapped;
    this.overlapped = false;
    return (!overlapped || text === this.sent) && this.sync(text);
  }

  /** Takes the saved `text` into the box when it may (see above); says whether it did. */
  sync(text: string): boolean {
    const own = text === this.sent;
    const untouched = this.latest === this.synced;
    if (this.waiting > 0 || !(own || (untouched && !this.focused))) {
      return false;
    }
    this.synced = text;
    if (!(untouched || (own && this.latest === this.sent))) {
      return false;
    }
    this.latest = text;
    return true;
  }
}
