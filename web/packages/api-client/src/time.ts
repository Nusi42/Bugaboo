/** The milliseconds in a second: a timer counts in them, the server in seconds. */
export const msPerSecond = 1000;

import { minutesPerHour, secondsPerMinute } from "./editing.generated.ts";

export { minutesPerHour, secondsPerMinute };

/** The seconds in an hour. */
export const secondsPerHour = secondsPerMinute * minutesPerHour;
