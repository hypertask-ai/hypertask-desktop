pub mod ai_chat;
pub mod desktop_cli;
pub mod auth;
mod catalog;
mod board_write;
pub use board_write::BoardWrite;
mod comment_write;
mod editor_queue;
pub use editor_queue::{DraftType, ImproveCommand, EditorResult};
mod comment_draft;
pub mod db;
pub mod generated;
pub mod http;
pub mod queue;
pub mod realtime;
pub mod sync;
mod task_patch;
mod shell_write;
pub mod worker;
mod sync_log;

pub use comment_write::CommentWrite;
pub use db::{BoardColumn, CommentView, Db, InboxItem, TaskDetail, TaskView};
pub use http::{Http, Transport};
pub use queue::{Drain, Write};
pub use sync::Sync;
pub use task_patch::TaskPatch;
pub use shell_write::ShellWrite;
pub use worker::Worker;

pub type Result<T> = anyhow::Result<T>;

mod inbox;
mod memory;
