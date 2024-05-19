#![no_std]
#![no_main]

use embassy_executor::Spawner;

// apparently sometimes RP2040 flashing hangs because of a bug in probe-rs
// https://github.com/probe-rs/probe-rs/pull/1603
// suggested workaround below
// https://matrix.to/#/!YoLPkieCYHGzdjUhOK:matrix.org/$JEW822NJW_aa6Juy3NCGpZWA0FcT0NJxtoEVIeTThxs
#[cortex_m_rt::pre_init]
unsafe fn before_main() {
    embassy_rp::pac::SIO.spinlock(31).write_value(1);
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    late_mate_firmware::main(spawner).await;
}
