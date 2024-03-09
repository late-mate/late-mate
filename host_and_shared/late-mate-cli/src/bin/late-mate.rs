#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    late_mate_cli::run().await
}
