use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct DispatchRequest {
    pub thread_id: String,
    pub parent_thread_id: String,
    pub agent_name: String,
    pub initial_input: String,
}

#[async_trait]
pub trait ThreadDispatcher: Send + Sync {
    async fn dispatch(&self, req: DispatchRequest) -> anyhow::Result<()>;
}
