/**
 * What is typed in a cell, and what of the week the server read again it may take over. The saved
 * text is taken (`sync`) only when no save of this cell waits, and then when it is the value sent
 * last, or when nothing newer was typed and the cell is not the one the person is in. So a text
 * that was read before the last save came back never replaces what was typed after it, and a
 * refused save leaves the draft where it was. When saves overlap, a text that came while they were
 * on their way may be older than the last of them, so it is taken when they are done only when it
 * means the value sent last (`same`: `2:30` for `2.5`); else only a text read after that can
 * replace what was typed. The read of the last save may well be drawn before that save answers.
 */
export class CellDraft {
  private latest: string;
  private synced: string;
  private sent: string | null = null;
  private waiting = 0;
  private overlapped = false;
  private focused = false;

  private same: (one: string, other: string) => boolean = (one, other) => one === other;

  constructor(text: string) {
    this.latest = text;
    this.synced = text;
  }

  /** Says whether two texts mean the same value; until it is given, only equal texts do. */
  compareWith(same: (one: string, other: string) => boolean): void {
    this.same = same;
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
    return (!overlapped || this.isSent(text)) && this.sync(text);
  }

  /** Takes the saved `text` into the box when it may (see above); says whether it did. */
  sync(text: string): boolean {
    // Only the very text sent: the server's way of writing it waits until the person is out of the cell.
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

  /** Whether `text` is the value sent last, maybe in the server's way of writing it. */
  private isSent(text: string): boolean {
    return this.sent !== null && this.same(text, this.sent);
  }
}
