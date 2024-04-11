import type { WsServer } from "../WsServer.ts";
import { HidReport } from "../generated/types/HidReport.ts";
import { Page } from "./page.ts";
import stringify from "json-stringify-pretty-compact";
import { assert } from "../utils.ts";

const WIDGETS: [string, string, HidReport][] = [
  [
    "input-mouse",
    "input-mouse-send",
    {
      type: "mouse",
      x: 70,
    },
  ],
  [
    "input-keyboard-1",
    "input-keyboard-1-send",
    {
      type: "keyboard",
      pressed_keys: ["a"],
    },
  ],
  [
    "input-keyboard-2",
    "input-keyboard-2-send",
    {
      type: "keyboard",
    },
  ],
];

export class RemotePage extends Page {
  readonly slug = "remote";

  constructor(server: WsServer) {
    super("menu-remote", "page-remote");

    for (const [inputId, sendId, hidReport] of WIDGETS) {
      const input = assert(
        document.getElementById(inputId),
      ) as HTMLTextAreaElement;
      const send = assert(document.getElementById(sendId)) as HTMLButtonElement;

      input.value = stringify(hidReport, { maxLength: 40 });

      send.addEventListener("click", () => {
        const hidReport = JSON.parse(input.value) as HidReport;
        server.send({
          type: "send_hid_report",
          hid_report: hidReport,
        });
      });
    }
  }
}
