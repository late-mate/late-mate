import { initSlider, initTextarea } from './thingies/latency_demo.ts';
import { initDiagram } from './thingies/device_diagram.ts';

// This is used just to make sure that Vite picks up and compiles the style.
// In dev mode, Vite injects it itself; in prod builds, Zola uses Vite's
// manifest to pick up the name with a hash
import './style.css';

initTextarea();
initSlider();
initDiagram();
