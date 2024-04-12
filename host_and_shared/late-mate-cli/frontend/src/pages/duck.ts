import type { WsServer } from "../WsServer.ts";
import { Page } from "./page.ts";
import { assert } from "../utils.ts";

const THRESHOLD = 0.1;

export class DuckPage extends Page {
  readonly slug = "duck";

  private readonly duckContainerId = "duck-container";
  private readonly duckCanvasId = "duck-canvas";
  private readonly duckFindId = "duck-find";
  private readonly duckContainer: HTMLDivElement;
  private readonly duckCanvas: HTMLCanvasElement;
  private readonly duckFindEl: HTMLButtonElement;
  private readonly duckCtx: CanvasRenderingContext2D;

  private lastSeenLight: number = 0;

  constructor(private readonly server: WsServer) {
    super("menu-duck", "page-duck");

    this.duckContainer = assert(
      document.getElementById(this.duckContainerId),
    ) as HTMLDivElement;
    this.duckCanvas = assert(
      document.getElementById(this.duckCanvasId),
    ) as HTMLCanvasElement;
    this.duckFindEl = assert(
      document.getElementById(this.duckFindId),
    ) as HTMLButtonElement;

    this.duckCanvas.width = this.duckContainer.clientWidth * 2;
    this.duckCanvas.height = this.duckContainer.clientHeight * 2;
    this.duckCanvas.style.width = `${this.duckContainer.clientWidth}px`;
    this.duckCanvas.style.height = `${this.duckContainer.clientHeight}px`;

    this.duckCtx = assert(this.duckCanvas.getContext("2d"));

    this.duckCtx.fillStyle = "black";
    this.duckCtx.fillRect(0, 0, this.duckCanvas.width, this.duckCanvas.height);

    this.server.subscribeToOpen(() => {
      if (this.isShown) {
        this.server.send({ type: "start_monitoring" });
      }
    });

    this.server.subscribe((msg) => {
      if (msg.type === "background_light_level" && this.isShown) {
        this.lastSeenLight = msg.avg;
      }
    });

    this.duckFindEl.addEventListener("click", () => {
      this.find();
    });
  }

  show() {
    super.show();
    if (this.server.isOpen) {
      this.server.send({ type: "start_monitoring" });
    }
  }

  hide() {
    super.hide();
    if (this.server.isOpen) {
      this.server.send({ type: "stop_monitoring" });
    }
  }

  // measurements come at 50hz; 25Hz sweep should be safe
  private readonly sweepDelay = 1000 / 25;
  private readonly sweepStep = 40;

  find() {
    console.log("find");
    this.sweepX();
  }

  sweepX() {
    this.duckCtx.fillStyle = "black";
    this.duckCtx.fillRect(0, 0, this.duckCanvas.width, this.duckCanvas.height);

    let isCooldown = true;
    let x = 0;

    const xInterval = setInterval(() => {
      if (isCooldown) {
        // skip one frame after refresh
        isCooldown = false;
        return;
      }

      console.log(this.lastSeenLight);

      if (this.lastSeenLight > THRESHOLD) {
        clearInterval(xInterval);
        console.log("duck found at x =", x);
        this.sweepY(x);
        return;
      }

      if (x >= this.duckCanvas.width) {
        clearInterval(xInterval);
        console.log("x sweep finished, no luck");
        return;
      }

      this.duckCtx.fillStyle = "black";
      this.duckCtx.fillRect(
        x - this.sweepStep,
        0,
        this.sweepStep,
        this.duckCanvas.height,
      );

      this.duckCtx.fillStyle = "white";
      this.duckCtx.fillRect(x, 0, this.sweepStep, this.duckCanvas.height);

      x += this.sweepStep;
    }, this.sweepDelay);
  }

  sweepY(x: number) {
    this.duckCtx.fillStyle = "black";
    this.duckCtx.fillRect(0, 0, this.duckCanvas.width, this.duckCanvas.height);

    let isCooldown = true;
    let y = 0;

    const yInterval = setInterval(() => {
      if (isCooldown) {
        // skip one frame after refresh
        isCooldown = false;
        return;
      }

      if (this.lastSeenLight > THRESHOLD) {
        clearInterval(yInterval);
        console.log("duck found at y =", y);
        this.duckFound(x, y);
        return;
      }

      if (y >= this.duckCanvas.height) {
        clearInterval(yInterval);
        console.log("y sweep finished, no luck");
        return;
      }

      this.duckCtx.fillStyle = "black";
      this.duckCtx.fillRect(
        0,
        y - this.sweepStep,
        this.duckCanvas.width,
        this.sweepStep,
      );

      this.duckCtx.fillStyle = "white";
      this.duckCtx.fillRect(0, y, this.duckCanvas.width, this.sweepStep);

      y += this.sweepStep;
    }, this.sweepDelay);
  }

  duckFound(x: number, y: number) {
    console.log("duck found at ", x, y);
    this.duckCtx.fillStyle = "black";
    this.duckCtx.fillRect(0, 0, this.duckCanvas.width, this.duckCanvas.height);

    this.duckCtx.font = "100px serif";
    this.duckCtx.fillText("🦆", x - this.sweepStep, y - this.sweepStep);
  }
}
