pub mod error;
pub mod spec;
pub mod context;
pub mod trait_def;
pub mod registry;
pub mod builtin;
pub mod mcp;

pub use error::ToolError;
pub use spec::ToolSpec;
pub use context::{ToolContext, ToolResult};
pub use trait_def::Tool;
pub use registry::ToolRegistry;
