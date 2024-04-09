//import viteLogo from '/vite.svg'
//import { setupCounter } from "./counter.ts";
import "./style.css";
import { ClientToServer } from "./generated/types/ClientToServer.ts";
import { ServerToClient } from "./generated/types/ServerToClient.ts";
import { HidReport } from "./generated/types/HidReport.ts";

// document.querySelector<HTMLDivElement>("#app")!.innerHTML = `
//   <div>
//     <a href="https://vitejs.dev" target="_blank">
//     vite
//     </a>
//     <a href="https://www.typescriptlang.org/" target="_blank">
//       <img src="${typescriptLogo}" class="logo vanilla" alt="TypeScript logo" />
//     </a>
//     <h1>Vite + TypeScript</h1>
//     <div class="card">
//       <button id="counter" type="button"></button>
//     </div>
//     <p class="read-the-docs">
//       Click on the Vite and TypeScript logos to learn more
//     </p>
//   </div>
// `;
//
// setupCounter(document.querySelector<HTMLButtonElement>("#counter")!);

class ServerWS {
  private ws: WebSocket;
  isOpen: boolean = false;
  msgListeners: ((msg: ServerToClient) => void)[] = [];

  constructor(url: string) {
    this.ws = new WebSocket(url);
    this.ws.addEventListener("open", () => {
      console.log("Connected to the server");
      this.isOpen = true;
    });

    this.ws.addEventListener("message", (evt) => {
      if (typeof evt.data === "string") {
        const msg = JSON.parse(evt.data) as ServerToClient;
        for (const listener of this.msgListeners) {
          listener(msg);
        }
      } else {
        console.error("Unexpected WS message", evt.data);
      }
    });

    this.ws.addEventListener("error", (evt) => {
      this.isOpen = false;
      console.error("WS error", evt);
    });

    this.ws.addEventListener("close", (evt) => {
      this.isOpen = false;
      console.log("WS closed", evt);
    });
  }

  send(msg: ClientToServer) {
    if (this.isOpen) {
      console.log("Sending a message", msg);
      this.ws.send(JSON.stringify(msg));
    } else {
      console.error("WS is closed, message can't be sent");
    }
  }

  subscribe(listener: (msg: ServerToClient) => void) {
    this.msgListeners.push(listener);
  }

  // unsubscribe(listener: (msg: ServerToClient) => void) {
  //   this.msgListeners = this.msgListeners.filter((l) => l !== listener);
  // }
}

const server = new ServerWS("ws://127.0.0.1:1838/ws");

// Status

document.getElementById("status-load")!.addEventListener("click", () => {
  server.send({ type: "status" });
});

server.subscribe((msg) => {
  if (msg.type === "status") {
    document.getElementById("status-pre")!.textContent = JSON.stringify(
      msg,
      null,
      2,
    );
  }
});

// Remote

const REMOTE_IDS: [string, string, HidReport][] = [
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

for (const [inputId, sendId, hidReport] of REMOTE_IDS) {
  const input = document.getElementById(inputId)! as HTMLTextAreaElement;
  const send = document.getElementById(sendId)! as HTMLButtonElement;

  input.value = JSON.stringify(hidReport, null, 2);

  send.addEventListener("click", () => {
    const hidReport = JSON.parse(input.value) as HidReport;
    server.send({
      type: "send_hid_report",
      hid_report: hidReport,
    });
  });
}
