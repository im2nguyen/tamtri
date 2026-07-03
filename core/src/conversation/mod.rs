pub mod meta;
pub mod model;
pub mod ops;
pub mod reduce;
pub mod roots;

pub use meta::{ConversationMeta, SCHEMA_VERSION, message_from_line, message_to_line};
pub use model::{
    ARTIFACT_INLINE_MAX_BYTES, ContentBlock, Conversation, ElicitationAction, ElicitationMode, Id,
    McpServerRef, Message, Role, Root, RootKind, RootScope, TaskStatus, WorkingDir,
};
pub use roots::{attach_root, is_path_under_any_root, is_path_under_root, remove_root, validate_root};
