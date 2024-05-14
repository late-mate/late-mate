use tokio::task::JoinSet;

pub fn start(mut agent_set: JoinSet<()>) {
    tokio::spawn(async move {
        let _ = agent_set
            .join_next()
            .await
            .expect("Agent JoinSet shouldn't be empty");
        tracing::warn!("One of the agents is dead, shuttind the rest down");
        agent_set.shutdown().await
    });
}
