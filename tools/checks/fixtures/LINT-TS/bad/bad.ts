export function risky(input: any, maybe: string | undefined): string {
  console.log(input);
  eval("1 + 1");
  return maybe!;
}

export function forgets(): void {
  Promise.resolve(1);
}

export default function defaulted(): number {
  return 1;
}
