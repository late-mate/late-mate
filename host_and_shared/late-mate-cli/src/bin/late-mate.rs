// note: using single-threaded Tokio due to https://github.com/berkowski/tokio-serial/issues/69
//       or maybe just handle it carefully on a separate thread/runtime?
// note: https://github.com/berkowski/tokio-serial/issues/55
// note: https://github.com/berkowski/tokio-serial/issues/37
#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    late_mate_cli::run().await
}
