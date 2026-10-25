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

mod ranking;
pub use ranking::task_rank;

pub mod task_create;

pub mod board_sidebar;

pub mod board_views;
pub use board_views::ViewWrite;

mod ai_sessions;
pub use ai_sessions::AiSessionWrite;

pub mod ai_context;

pub mod preferences;
pub mod ai_models;

pub mod team_settings;

pub mod smart_splits;

pub mod board_columns;

pub mod task_time;
pub use task_time::TimeWrite;

pub mod task_actions;
pub use task_actions::TaskAction;
mod memory;

pub mod task_relations;
pub use task_relations::RelationKind;

pub mod compose_routing;

pub mod time_entries;
pub use time_entries::TimeEntryChange;

pub mod date_options;
