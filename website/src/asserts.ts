export function assertDefined<T>(x: T | undefined | null): asserts x is T {
  if (x === undefined || x === null) {
    throw new Error(`Unexpected ${x}`);
  }
}

export function elById<T extends keyof HTMLElementTagNameMap>(
  tag: T,
  id: string,
): HTMLElementTagNameMap[T] | null {
  const el = document.getElementById(id);

  if (!el) {
    return el;
  }

  if (el.tagName.toLowerCase() !== tag.toLowerCase()) {
    throw new Error(`Expected ${tag}#${id}, got ${el.tagName}#${id}`);
  }
  return el as HTMLElementTagNameMap[T];
}

export function assertEl<T extends keyof HTMLElementTagNameMap>(
  tag: T,
  el: any,
): HTMLElementTagNameMap[T] {
  if (!(el instanceof HTMLElement)) {
    throw new Error(`Expected ${el} to be an HTMLElement`);
  }
  if (el.tagName.toLowerCase() !== tag.toLowerCase()) {
    throw new Error(`Expected ${el} to be ${tag}`);
  }
  return el as HTMLElementTagNameMap[T];
}
