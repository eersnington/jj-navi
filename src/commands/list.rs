use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::Result;
use crate::output::{render_workspace_list_json, render_workspace_table};
use crate::repo::NaviWorkspace;
use crate::types::{WorkspaceFreshnessStatus, WorkspaceListEntry};

/// Run the `list` command.
///
/// # Errors
///
/// Returns an error if workspace discovery fails or if `jj workspace list`
/// fails.
pub fn run_list(path: &Path, json: bool, compact: bool) -> Result<()> {
    let repo = NaviWorkspace::open(path)?;

    if json {
        let snapshots = repo.list_fresh_workspace_snapshots()?;
        println!(
            "{}",
            render_workspace_list_json(repo.workspace_root(), &snapshots, compact)?
        );
        return Ok(());
    }

    let spinner = ListSpinner::start();
    let entries = repo.list_workspaces()?;
    spinner.stop();

    print!("{}", render_workspace_table(&entries));
    warn_if_any_workspace_is_not_current(&entries);
    Ok(())
}

fn warn_if_any_workspace_is_not_current(entries: &[WorkspaceListEntry]) {
    for entry in entries.iter().filter(|entry| {
        matches!(
            entry.freshness.status,
            WorkspaceFreshnessStatus::Failed | WorkspaceFreshnessStatus::TimedOut
        )
    }) {
        eprintln!(
            "warning: workspace '{}' could not be made current{}",
            entry.name,
            entry
                .freshness
                .reason
                .as_ref()
                .map_or_else(String::new, |reason| format!(": {reason}"))
        );
    }
}

struct ListSpinner {
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl ListSpinner {
    fn start() -> Self {
        if !std::io::IsTerminal::is_terminal(&std::io::stderr()) {
            return Self {
                running: Arc::new(AtomicBool::new(false)),
                handle: None,
            };
        }

        let running = Arc::new(AtomicBool::new(true));
        let spinner_running = Arc::clone(&running);
        let handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            let frames = ["|", "/", "-", "\\"];
            let mut frame_index = 0;

            while spinner_running.load(Ordering::Relaxed) {
                eprint!(
                    "\r{} Refreshing workspaces...",
                    frames[frame_index % frames.len()]
                );
                let _ = std::io::Write::flush(&mut std::io::stderr());
                frame_index += 1;
                std::thread::sleep(Duration::from_millis(100));
            }
        });

        Self {
            running,
            handle: Some(handle),
        }
    }

    fn stop(mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
            eprint!("\r\x1b[2K");
            let _ = std::io::Write::flush(&mut std::io::stderr());
        }
    }
}

impl Drop for ListSpinner {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
            eprint!("\r\x1b[2K");
            let _ = std::io::Write::flush(&mut std::io::stderr());
        }
    }
}
