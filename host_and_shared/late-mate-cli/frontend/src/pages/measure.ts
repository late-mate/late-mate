import type { WsServer } from "../WsServer.ts";
import { Page } from "./page.ts";
import { ClientToServer } from "../generated/types/ClientToServer.ts";
import stringify from "json-stringify-pretty-compact";

import Chart, { ChartConfiguration, Point } from "chart.js/auto";
import annotationPlugin from "chartjs-plugin-annotation";
import { ServerToClient } from "../generated/types/ServerToClient.ts";
import { assert } from "../utils.ts";

Chart.register(annotationPlugin);

type PresetScenario = {
  buttonId: string;
  scenario: ClientToServer & { type: "measure" };
};

const PRESET_SCENARIOS: PresetScenario[] = [
  {
    buttonId: "measure-scenario-a",
    scenario: {
      type: "measure",
      duration_ms: 300,
      before: [],
      start: { type: "keyboard", pressed_keys: ["a"] },
      followup: {
        after_ms: 1,
        hid_report: { type: "keyboard" },
      },
      after: [],
    },
  },
  {
    buttonId: "measure-scenario-draw",
    scenario: {
      type: "measure",
      duration_ms: 300,
      before: [{ type: "mouse", buttons: ["left"] }],
      start: { type: "mouse", buttons: ["left"], x: 120 },
      followup: null,
      after: [
        { type: "mouse" },
        { type: "keyboard", pressed_keys: ["z"], modifiers: ["l_meta"] },
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
        pointRadius: 5,
        pointBorderWidth: 0,
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
  private readonly chartAreaId = "measure-chart-area";
  private readonly currentCanvasId = "measure-current-canvas";
  private readonly statCanvasId = "measure-stat-canvas";

  private readonly inputEl: HTMLTextAreaElement;
  private readonly sendEl: HTMLButtonElement;
  private readonly sendX10El: HTMLButtonElement;
  private readonly chartAreaEl: HTMLDivElement;

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
    this.chartAreaEl = assert(
      document.getElementById(this.chartAreaId),
    ) as HTMLDivElement;

    for (const { buttonId, scenario } of PRESET_SCENARIOS) {
      assert(document.getElementById(buttonId)).addEventListener(
        "click",
        () => {
          this.inputEl.value = stringify(scenario, { maxLength: 40 });
        },
      );
    }

    this.chartAreaEl.classList.add("hidden");

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
      let toGo = 10;
      const sendNext = () => {
        if (toGo === 0) {
          this.x10InFlight = false;
          return;
        }

        toGo -= 1;
        this.sendRequest(scenario);
        setTimeout(sendNext, scenario.duration_ms * 1.5);
      };
      sendNext();
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

    this.currentChart.data.datasets![0].data = msg.light_levels.map((x) => ({
      x: x[0] / 1000,
      y: (x[1] / msg.max_light_level) * 100,
    }));

    let annotation = (
      this.currentChart.options.plugins!.annotation!.annotations as any
    )["change"];
    annotation["xMin"] = msg.change_us / 1000;
    annotation["xMax"] = msg.change_us / 1000;

    if (this.isScenarioNew) {
      this.isScenarioNew = false;
      this.statChart.data.datasets![0].data = [];
    }

    this.currentChart.options.scales!.x!.max = this.scenarioDuration;
    this.statChart.options.scales!.x!.max = this.scenarioDuration;

    this.statChart.data.datasets![0].data.push({
      x: msg.change_us / 1000,
      y: Math.random(),
    });

    this.currentChart.update("none");
    this.statChart.update("none");

    this.chartAreaEl.classList.remove("hidden");
  }
}
