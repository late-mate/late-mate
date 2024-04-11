import type { WsServer } from "../WsServer.ts";
import { Page } from "./page.ts";
import stringify from "json-stringify-pretty-compact";

import Chart, { ChartConfiguration, Point } from "chart.js/auto";
import annotationPlugin from "chartjs-plugin-annotation";
import { assert } from "../utils.ts";
import { ClientToServer } from "../generated/types/ClientToServer.ts";
import { ServerToClient } from "../generated/types/ServerToClient.ts";

Chart.register(annotationPlugin);

type PresetScenario = {
  buttonId: string;
  scenario: ClientToServer & { type: "measure" };
};

const PRESET_SCENARIOS: PresetScenario[] = [
  {
    buttonId: "measure-scenario-a",
    // scenario: {
    //   type: "measure",
    //   duration_ms: 300,
    //   before: [],
    //   start: { type: "keyboard", pressed_keys: ["a"] },
    //   followup: {
    //     after_ms: 1,
    //     hid_report: { type: "keyboard" },
    //   },
    //   after: [],
    // },
    scenario: {
      type: "measure",
      duration_ms: 300,
      before: [],
      start: { type: "keyboard", pressed_keys: ["a"] },
      followup: {
        after_ms: 1,
        hid_report: { type: "keyboard" },
      },
      after: [
        { type: "keyboard", pressed_keys: ["backspace"] },
        { type: "keyboard" },
      ],
    },
  },
  {
    buttonId: "measure-scenario-draw",
    scenario: {
      type: "measure",
      duration_ms: 150,
      before: [],
      start: { type: "mouse", buttons: ["left"] },
      followup: { after_ms: 1, hid_report: { type: "mouse" } },
      after: [
        { type: "keyboard", modifiers: ["l_meta"] },
        { type: "keyboard", modifiers: ["l_meta"], pressed_keys: ["z"] },
        { type: "keyboard" },
      ],
    },
  },
  {
    buttonId: "measure-scenario-doom",
    scenario: {
      type: "measure",
      duration_ms: 300,
      before: [],
      start: { type: "mouse", y: -80 },
      followup: null,
      after: [{ type: "mouse", y: 80 }],
    },
  },
];

const CURRENT_CHART_INIT_CONFIG: ChartConfiguration<"line", Point[], string> = {
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
      annotation: {
        annotations: {
          change: {
            display: false,
            type: "line",
            borderWidth: 1,
            borderColor: "red",
            xMin: 0,
            xMax: 0,
            label: {
              display: true,
              content: "Change",
              rotation: 270,
              backgroundColor: "transparent",
              color: "red",
              position: "end",
            },
          },
        },
      },
    },
    scales: {
      x: {
        type: "linear",
        title: {
          text: "Time (ms)",
          display: true,
        },
        min: 0,
        max: 300,
      },
      y: {
        type: "linear",
        display: false,
        title: {
          text: "Light level (%)",
          display: true,
        },
        //min: 0,
        //max: 100,
      },
    },
  },
};

const START_CHART_CONFIG: ChartConfiguration<"scatter", Point[], string> = {
  type: "scatter",
  data: {
    datasets: [
      {
        data: [],
        pointRadius: 3,
        pointBorderWidth: 0,
        pointBackgroundColor: "rgba(55,162,235,0.4)",
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
        title: {
          text: "Delay (ms)",
          display: true,
        },
        min: 0,
        max: 300,
      },
      y: {
        type: "linear",
        display: false,
        max: 3,
        min: -2,
      },
    },
  },
};

export class MeasurePage extends Page {
  readonly slug = "measure";

  private readonly inputId = "input-measure-scenario";
  private readonly sendId = "input-measure-send";
  private readonly sendX10Id = "input-measure-send-x10";
  private readonly enoughId = "input-measure-enough";
  private readonly chartAreaId = "measure-chart-area";
  private readonly currentCanvasId = "measure-current-canvas";
  private readonly statContainerId = "measure-stat-container";
  private readonly statCanvasId = "measure-stat-canvas";

  private readonly inputEl: HTMLTextAreaElement;
  private readonly sendEl: HTMLButtonElement;
  private readonly sendX10El: HTMLButtonElement;
  private readonly enoughEl: HTMLButtonElement;
  private readonly chartAreaEl: HTMLDivElement;
  private readonly statContainerEl: HTMLDivElement;

  private readonly currentChart: Chart<"line", Point[], string>;
  private readonly statChart: Chart<"scatter", Point[], string>;

  private readonly server: WsServer;

  private lastScenarioJson: string = "";
  private isScenarioNew: boolean = false;
  private scenarioDuration: number = 0;

  private requestInFlight: boolean = false;
  private x10InFlight: boolean = false;

  constructor(server: WsServer) {
    super("menu-measure", "page-measure");

    this.server = server;

    this.inputEl = assert(
      document.getElementById(this.inputId),
    ) as HTMLTextAreaElement;
    this.sendEl = assert(
      document.getElementById(this.sendId),
    ) as HTMLButtonElement;
    this.sendX10El = assert(
      document.getElementById(this.sendX10Id),
    ) as HTMLButtonElement;
    this.enoughEl = assert(
      document.getElementById(this.enoughId),
    ) as HTMLButtonElement;
    this.chartAreaEl = assert(
      document.getElementById(this.chartAreaId),
    ) as HTMLDivElement;
    this.statContainerEl = assert(
      document.getElementById(this.statContainerId),
    ) as HTMLDivElement;

    for (const { buttonId, scenario } of PRESET_SCENARIOS) {
      assert(document.getElementById(buttonId)).addEventListener(
        "click",
        () => {
          this.inputEl.value = stringify(scenario, { maxLength: 40 });
        },
      );
    }

    //this.chartAreaEl.classList.add("hidden");

    this.sendEl.addEventListener("click", () => {
      if (this.requestInFlight || this.x10InFlight || !this.inputEl.value) {
        return;
      }

      const scenario = JSON.parse(this.inputEl.value) as ClientToServer;
      if (scenario.type !== "measure") {
        throw new Error("Scenario can only of type 'Measure'");
      }

      this.sendRequest(scenario);
    });

    this.sendX10El.addEventListener("click", () => {
      if (this.requestInFlight || this.x10InFlight || !this.inputEl.value) {
        return;
      }

      const scenario = JSON.parse(this.inputEl.value) as ClientToServer;
      if (scenario.type !== "measure") {
        throw new Error("Scenario can only of type 'Measure'");
      }

      this.x10InFlight = true;
      let toGo = 50;
      const sendNext = () => {
        if (toGo === 0) {
          this.x10InFlight = false;
          return;
        }

        toGo -= 1;
        this.sendRequest(scenario);
        setTimeout(sendNext, 500);
      };
      sendNext();
    });

    this.enoughEl.addEventListener("click", () => {
      const newImage = this.statChart.toBase64Image();
      const img = document.createElement("img");
      img.src = newImage;
      img.style.width = this.statChart.canvas.style.width;
      img.style.height = this.statChart.canvas.style.height;

      // insert img as a second child of the container
      this.statContainerEl.insertBefore(
        img,
        this.statContainerEl.childNodes[1] ?? null,
      );

      this.currentChart.data.datasets![0].data = [];
      this.currentChart.update("none");

      this.statChart.data.datasets![0].data = [];
      this.statChart.update("none");
    });

    this.currentChart = new Chart(
      assert(
        document.getElementById(this.currentCanvasId),
      ) as HTMLCanvasElement,
      CURRENT_CHART_INIT_CONFIG,
    );

    this.statChart = new Chart(
      assert(document.getElementById(this.statCanvasId)) as HTMLCanvasElement,
      START_CHART_CONFIG,
    );

    server.subscribe((msg) => {
      if (msg.type === "measurement") {
        this.processMeasurement(msg);
      }
    });
  }

  sendRequest(scenario: ClientToServer & { type: "measure" }) {
    const scenarioJson = JSON.stringify(scenario);
    if (scenarioJson !== this.lastScenarioJson) {
      this.isScenarioNew = true;
      this.chartAreaEl.classList.add("hidden");
      this.lastScenarioJson = scenarioJson;
      this.scenarioDuration = scenario.duration_ms;
    }

    this.requestInFlight = true;
    this.server.send(scenario);
  }

  processMeasurement(msg: ServerToClient & { type: "measurement" }) {
    this.requestInFlight = false;

    if (this.isScenarioNew) {
      this.isScenarioNew = false;
      this.statChart.data.datasets![0].data = [];
    }

    this.currentChart.data.datasets![0].data = msg.light_levels.map(
      (x: [number, number]) => ({
        x: x[0] / 1000,
        y: (x[1] / msg.max_light_level) * 100,
      }),
    );

    let annotation = (
      this.currentChart.options.plugins!.annotation!.annotations as any
    )["change"];

    if (msg.change_us === null) {
      annotation.display = false;
    } else {
      annotation.display = true;
      annotation.xMin = msg.change_us / 1000;
      annotation.xMax = msg.change_us / 1000;

      this.statChart.data.datasets![0].data.push({
        x: msg.change_us / 1000,
        y: Math.random(),
      });
    }

    this.currentChart.options.scales!.x!.max = this.scenarioDuration;
    this.statChart.options.scales!.x!.max = this.scenarioDuration;

    this.currentChart.update("none");
    this.statChart.update("none");

    this.chartAreaEl.classList.remove("hidden");
  }
}
