export function assert<T>(x: T | undefined | null): T {
  if (x === undefined || x === null) {
    throw new Error(`Unexpected ${x}`);
  }
  return x;
}
