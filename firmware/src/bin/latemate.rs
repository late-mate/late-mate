#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use embassy_executor::Spawner;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    latematefw::main(spawner).await;
}
