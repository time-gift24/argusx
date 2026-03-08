#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub struct AppConfig;

pub struct ArgusxRuntime;

pub async fn build_runtime() -> anyhow::Result<ArgusxRuntime> {
    anyhow::bail!("not implemented")
}
