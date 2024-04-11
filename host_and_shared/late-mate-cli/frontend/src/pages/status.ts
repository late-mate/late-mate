import type { WsServer } from "../WsServer.ts";
import { Page } from "./page.ts";
import { assert } from "../utils.ts";
import stringify from "json-stringify-pretty-compact";

export class StatusPage extends Page {
  readonly slug = "status";

  private readonly statusPreId = "status-pre";
  private readonly statusEl: HTMLPreElement;

  constructor(server: WsServer) {
    super("menu-status", "page-status");

    this.statusEl = assert(
      document.getElementById(this.statusPreId),
    ) as HTMLPreElement;

    assert(document.getElementById("status-load")).addEventListener(
      "click",
      () => {
        server.send({ type: "status" });
      },
    );

    server.subscribe((msg) => {
      if (msg.type === "status") {
        this.statusEl.textContent = stringify(msg, { maxLength: 40 });
      }
    });
  }
}
