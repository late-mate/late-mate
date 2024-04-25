## How to develop the CLI server stuff

1. run `RUST_LOG=debug cargo run --release --bin late-mate run-server`
2. run `yarn dev` in `frontend/` (in a separate tab), this will run a Vite dev server. Add
   `--host` as needed
3. point your browser at Vite's interface/port, it will proxy the websocket to the CLI