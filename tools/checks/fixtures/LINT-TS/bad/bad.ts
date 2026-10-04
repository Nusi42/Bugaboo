export function risky(input: any, maybe: string | undefined): string {
  console.log(input);
  eval("1 + 1");
  return maybe!;
}
