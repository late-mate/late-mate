import "./style.css";
import { WsServer } from "./WsServer.ts";
import { StatusPage } from "./pages/status.ts";
import { RemotePage } from "./pages/remote.ts";
import { Page } from "./pages/page.ts";
import { MeasurePage } from "./pages/measure.ts";
import { MonitorPage } from "./pages/monitor.ts";
import { DuckPage } from "./pages/duck.ts";

const wsUrl = new URL("./ws", location.href);
wsUrl.protocol = wsUrl.protocol.replace("http", "ws");
const server = new WsServer(wsUrl.toString());

const PAGES: Page[] = [
  new StatusPage(server),
  new MonitorPage(server),
  new RemotePage(server),
  new MeasurePage(server),
  new DuckPage(server)
];

// Routing
let activePage = PAGES[0];

if (window.location.hash) {
  const slug = window.location.hash.slice(1);
  const page = PAGES.find((p) => p.slug === slug);
  if (page) {
    activePage = page;
  }
}

for (const page of PAGES) {
  if (page === activePage) {
    page.show();
  } else {
    page.hide();
  }

  page.menuEl.addEventListener("click", () => {
    if (activePage === page) {
      return;
    }
    activePage.hide();
    page.show();
    window.location.hash = `#${page.slug}`;
    activePage = page;
  });
}
