mod domain;
mod error;
mod repository;
mod service;
mod sqlite;

use std::sync::Arc;

use argusx_common::config::Settings;

pub use domain::*;
pub use error::{PromptLabError, Result};
pub use service::{AiLogService, CheckResultService, ChecklistService, GoldenSetService, SopService};
pub use sqlite::{DbConfig, PragmaStatus};

use repository::PromptLabRepository;

#[derive(Clone)]
pub struct PromptLab {
    repo: Arc<PromptLabRepository>,
    checklist_service: ChecklistService,
    golden_set_service: GoldenSetService,
    check_result_service: CheckResultService,
    ai_log_service: AiLogService,
    sop_service: SopService,
}

impl PromptLab {
    pub async fn new(settings: Settings) -> Result<Self> {
        let pool = sqlite::connect(&settings.database).await?;
        sqlite::run_migrations(&pool).await?;

        let repo = Arc::new(PromptLabRepository::new(pool));

        Ok(Self {
            repo: repo.clone(),
            checklist_service: ChecklistService::new(repo.clone()),
            golden_set_service: GoldenSetService::new(repo.clone()),
            check_result_service: CheckResultService::new(repo.clone()),
            ai_log_service: AiLogService::new(repo.clone()),
            sop_service: SopService::new(repo),
        })
    }

    pub fn checklist_service(&self) -> ChecklistService {
        self.checklist_service.clone()
    }

    pub fn golden_set_service(&self) -> GoldenSetService {
        self.golden_set_service.clone()
    }

    pub fn check_result_service(&self) -> CheckResultService {
        self.check_result_service.clone()
    }

    pub fn ai_log_service(&self) -> AiLogService {
        self.ai_log_service.clone()
    }

    pub fn sop_service(&self) -> SopService {
        self.sop_service.clone()
    }

    pub async fn pragma_status(&self) -> Result<PragmaStatus> {
        sqlite::pragma_status(self.repo.pool()).await
    }
}
