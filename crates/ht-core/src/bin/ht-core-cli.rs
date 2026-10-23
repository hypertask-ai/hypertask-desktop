use anyhow::{Context, bail, ensure};
use ht_core::{Db, Drain, Http, Result, Sync, Write};

const SYNTHETIC_ID_BASE: i64 = 900_000_000;

fn read_json(path: &std::path::Path) -> Result<serde_json::Value> {
    Ok(serde_json::from_slice(&std::fs::read(path)?).with_context(|| format!("{}", path.display()))?)
}

fn import_fixtures(db: &mut Db, dir: &std::path::Path) -> Result<usize> {
    let mut files: Vec<_> = std::fs::read_dir(dir)?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<std::io::Result<_>>()?;
    files.retain(|path| path.extension().is_some_and(|ext| ext == "json"));
    files.sort();
    let (comment_files, response_files): (Vec<_>, Vec<_>) = files.into_iter().partition(|path| {
        path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("comments-"))
    });
    for path in &response_files {
        db.apply_response(&read_json(path)?)?;
    }
    for path in &comment_files {
        let task_id: i64 = path
            .file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| s.strip_prefix("comments-"))
            .context("comments fixture name must be comments-<task id>.json")?
            .parse()?;
        db.apply_comments(task_id, &read_json(path)?)?;
    }
    Ok(response_files.len() + comment_files.len())
}

fn add_synthetic_tasks(db: &mut Db, dir: &std::path::Path, count: usize) -> Result<usize> {
    if count == 0 {
        return Ok(0);
    }
    let templates = read_json(&dir.join("tasks-15-0.json"))?["tasks"]
        .as_array()
        .context("tasks-15-0.json has no tasks")?
        .clone();
    let section_ids: Vec<_> = read_json(&dir.join("sections-15.json"))?["sections"]
        .as_array()
        .context("sections-15.json has no sections")?
        .iter()
        .map(|s| (s["id"].clone(), s["section_title"].clone()))
        .collect();
    ensure!(!templates.is_empty() && !section_ids.is_empty(), "fixtures have no tasks or sections");
    let tasks: Vec<_> = (0..count)
        .map(|i| {
            let mut task = templates[i % templates.len()].clone();
            let (section_id, section_title) = &section_ids[i % section_ids.len()];
            let id = SYNTHETIC_ID_BASE + i as i64;
            task["id"] = id.into();
            task["uniqueIndex"] = id.into();
            task["ticketNumber"] = format!("SYN-{i}").into();
            task["title"] = format!("{} #{i}", task["title"].as_str().unwrap_or("Task")).into();
            task["sectionId"] = section_id.clone();
            task["section"] = section_title.clone();
            task
        })
        .collect();
    db.apply_response(&serde_json::json!({ "success": true, "tasks": tasks }))?;
    Ok(count)
}

fn run() -> Result<()> {
    let mut args: Vec<_> = std::env::args().skip(1).collect();
    let mut path = std::env::var("HT_DB").unwrap_or_else(|_| "hypertask.sqlite3".into());
    if let Some(index) = args.iter().position(|s| s == "--db") {
        ensure!(index + 1 < args.len(), "--db requires a path");
        path = args.remove(index + 1);
        args.remove(index);
    }
    let db = Db::open(path)?;
    match args.first().map(String::as_str) {
        Some("dump") if args.len() == 1 => {
            println!("{}", serde_json::to_string_pretty(&db.dump()?)?)
        }
        Some("sync") if args.len() == 3 && args[1] == "--board" => {
            let board_id = args[2].parse::<i64>()?;
            let http =
                Http::new(std::env::var("HT_TOKEN").context("HT_TOKEN is required for sync")?)?;
            std::thread::spawn(move || -> Result<()> {
                let mut sync = Sync::new(db, http);
                loop {
                    match sync
                        .db
                        .drain_once(&mut sync.transport, chrono::Utc::now().timestamp())?
                    {
                        Drain::Applied(_) | Drain::Failed(_) => continue,
                        _ => break,
                    }
                }
                sync.projects()?;
                sync.board(board_id)?;
                sync.inbox()?;
                sync.refetch_pending()?;
                Ok(())
            })
            .join()
            .map_err(|_| anyhow::anyhow!("sync thread failed"))??;
            println!("SYNC_OK");
        }
        Some("write") if args.len() >= 3 => {
            let id: i64 = args[2].parse()?;
            let write = match args[1].as_str() {
                "task_title" if args.len() == 4 => Write::TaskTitle {
                    task_id: id,
                    title: args[3].clone(),
                },
                "task_section" if args.len() == 4 => Write::TaskSection {
                    task_id: id,
                    section_id: args[3].parse()?,
                },
                "task_comment" if args.len() == 4 => Write::TaskComment {
                    task_id: id,
                    text: args[3].clone(),
                },
                "inbox_archive" if args.len() == 3 => Write::InboxArchive {
                    notification_id: id,
                },
                _ => bail!(
                    "write kinds: task_title <task-id> <title>, task_section <task-id> <section-id>, task_comment <task-id> <text>, inbox_archive <notification-id>"
                ),
            };
            let mut db = db;
            let mutation = db.enqueue(write, chrono::Utc::now().timestamp())?;
            println!("QUEUED {mutation}");
            if let Ok(token) = std::env::var("HT_TOKEN") {
                let mut http = Http::new(token)?;
                std::thread::spawn(move || {
                    db.drain_once(&mut http, chrono::Utc::now().timestamp())
                })
                .join()
                .map_err(|_| anyhow::anyhow!("write thread failed"))??;
            }
        }
        Some("import-fixtures") if args.len() == 2 || args.len() == 4 => {
            let synthetic_tasks = match args.get(2).map(String::as_str) {
                Some("--synthetic-tasks") => args[3].parse::<usize>()?,
                None => 0,
                Some(other) => bail!("unknown import-fixtures option {other}"),
            };
            let mut db = db;
            let imported = import_fixtures(&mut db, std::path::Path::new(&args[1]))?;
            let added = add_synthetic_tasks(&mut db, std::path::Path::new(&args[1]), synthetic_tasks)?;
            println!("IMPORTED files={imported} synthetic_tasks={added}");
        }
        _ => bail!(
            "usage: ht-core-cli [--db path] sync --board <id> | write <kind> ... | dump | import-fixtures <dir> [--synthetic-tasks <n>]"
        ),
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
