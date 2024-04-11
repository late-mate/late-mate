import { assert } from "../utils.ts";

const MENU_ACTIVE_CLASSES = ["underline", "underline-offset-2"];

export abstract class Page {
  abstract readonly slug: string;

  readonly menuEl: HTMLElement;
  readonly pageEl: HTMLDivElement;

  constructor(menuId: string, pageElementId: string) {
    this.menuEl = assert(document.getElementById(menuId));
    this.pageEl = assert(
      document.getElementById(pageElementId),
    ) as HTMLDivElement;
  }

  show(): void {
    this.pageEl.classList.remove("hidden");
    this.menuEl.classList.add(...MENU_ACTIVE_CLASSES);
  }

  hide(): void {
    this.pageEl.classList.add("hidden");
    this.menuEl.classList.remove(...MENU_ACTIVE_CLASSES);
  }
}
