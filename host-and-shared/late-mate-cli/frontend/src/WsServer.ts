import { ServerToClient } from "./generated/types/ServerToClient.ts";
import { ClientToServer } from "./generated/types/ClientToServer.ts";

export class WsServer {
  private ws: WebSocket;
  isOpen: boolean = false;
  msgListeners: ((msg: ServerToClient) => void)[] = [];
  openListeners: (() => void)[] = [];

  constructor(url: string) {
    this.ws = new WebSocket(url);
    this.ws.addEventListener("open", () => {
      console.log("Connected to the server");
      this.isOpen = true;
      for (const listener of this.openListeners) {
        listener();
      }
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

  subscribeToOpen(listener: () => void) {
    this.openListeners.push(listener);
  }

  // unsubscribe(listener: (msg: ServerToClient) => void) {
  //   this.msgListeners = this.msgListeners.filter((l) => l !== listener);
  // }
}
