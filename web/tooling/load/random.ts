/** A source of numbers from 0 up to 1 that is the same every time for one seed (mulberry32). */
export function randomFrom(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d_2b_79_f5) >>> 0;
    let mixed = Math.imul(state ^ (state >>> 15), state | 1);
    mixed ^= mixed + Math.imul(mixed ^ (mixed >>> 7), mixed | 61);
    return ((mixed ^ (mixed >>> 14)) >>> 0) / 4_294_967_296;
  };
}

/** A whole number from 0 up to, not including, `below`. */
export function pick(random: () => number, below: number): number {
  return Math.floor(random() * below);
}
