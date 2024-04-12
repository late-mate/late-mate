import type { WsServer } from "../WsServer.ts";
import { Page } from "./page.ts";
import { assert } from "../utils.ts";
import Chart, { Point } from "chart.js/auto";

export class MonitorPage extends Page {
  readonly slug = "monitor";

  private readonly chartId = "monitor-chart-canvas";
  private readonly chartEl: HTMLCanvasElement;
  private readonly chart: Chart<"line", Point[], string>;

  constructor(private readonly server: WsServer) {
    super("menu-monitor", "page-monitor");

    this.chartEl = assert(
      document.getElementById(this.chartId),
    ) as HTMLCanvasElement;

    this.chart = new Chart(this.chartEl, {
      type: "line",
      data: {
        datasets: [
          {
            borderWidth: 1,
            pointStyle: false,
            pointRadius: 0,
            data: [],
            parsing: false,
          },
        ],
      },
      options: {
        animation: false,

        plugins: {
          legend: {
            display: false,
          },
          tooltip: {
            enabled: false,
          },
        },
        scales: {
          x: {
            type: "linear",
            display: false,
            min: 0,
            max: 500,
          },
          y: {
            type: "linear",
            display: true,
            title: {
              text: "Light level (%)",
              display: true,
            },
            min: 0,
            max: 100,
          },
        },
      },
    });

    this.server.subscribeToOpen(() => {
      if (this.isShown) {
        this.server.send({ type: "start_monitoring" });
      }
    });

    this.server.subscribe((msg) => {
      if (msg.type === "background_light_level") {
        if (this.chart.data.datasets![0].data.length >= 500) {
          this.chart.data.datasets![0].data.shift();
        }
        this.chart.data.datasets![0].data.push({ x: 0, y: msg.avg * 100 });
        for (let i = 0; i < this.chart.data.datasets![0].data.length; i++) {
          this.chart.data.datasets![0].data[i].x = i;
        }
        this.chart.update("none");
      }
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
    this.chart.data.datasets![0].data = [];
  }
}
