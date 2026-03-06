#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    Allow,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnCommand {
    Cancel,
    ResolvePermission {
        request_id: String,
        decision: PermissionDecision,
    },
}
