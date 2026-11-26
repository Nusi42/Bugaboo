/**
 * The keys that move an issue picked up on the calendar (PLAN-09, TEST-A11Y): the left and right arrows
 * a day, the up and down arrows a week, and *Shift* with the left and right arrows a week too. Every
 * drag of the calendar is also possible this way, or with *Change due date …*.
 */

const daysPerWeek = 7;

/** The days a key moves a picked issue by, or none for a key that moves nothing. */
export function stepOfKey(key: string, shift: boolean): number | undefined {
  switch (key) {
    case "ArrowLeft":
      return shift ? -daysPerWeek : -1;
    case "ArrowRight":
      return shift ? daysPerWeek : 1;
    case "ArrowUp":
      return -daysPerWeek;
    case "ArrowDown":
      return daysPerWeek;
    default:
      return undefined;
  }
}
