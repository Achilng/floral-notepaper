use super::{commands, settings, types::UpdateErrorDto, UpdatePaths, UpdaterState};
use crate::services::notes::AppError;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use std::{thread, time::Duration};
use tauri::{AppHandle, Emitter, Manager};

const INITIAL_DELAY: Duration = Duration::from_secs(3);
const POLL_INTERVAL: Duration = Duration::from_secs(60);

pub fn start_auto_check_scheduler(app: AppHandle) {
    thread::spawn(move || {
        thread::sleep(INITIAL_DELAY);

        loop {
            if let Err(error) = poll_auto_check(&app, Utc::now()) {
                eprintln!("failed to run automatic update check: {error}");
                let payload =
                    UpdateErrorDto::recoverable(error.code, error.message, Some("retry".into()));
                if let Err(emit_error) = app.emit("update://auto-check-error", payload) {
                    eprintln!("failed to emit update://auto-check-error: {emit_error}");
                }
            }

            thread::sleep(POLL_INTERVAL);
        }
    });
}

fn poll_auto_check(app: &AppHandle, now: DateTime<Utc>) -> Result<(), AppError> {
    let Some(state) = app.try_state::<UpdaterState>() else {
        return Ok(());
    };

    let _ = maybe_run_due_check(state.paths(), now, || {
        commands::run_automatic_update_check(app.clone(), state.inner()).map(|_| ())
    })?;

    Ok(())
}

fn should_auto_check(settings: &settings::StoredUpdateSettings, now: DateTime<Utc>) -> bool {
    if !settings.auto_check {
        return false;
    }

    let Some(last_checked_at) = settings.last_auto_check_at else {
        return true;
    };

    let interval = ChronoDuration::hours(i64::from(settings.check_interval_hours));
    now.signed_duration_since(last_checked_at) >= interval
}

pub(crate) fn maybe_run_due_check<F>(
    paths: &UpdatePaths,
    now: DateTime<Utc>,
    mut runner: F,
) -> Result<bool, AppError>
where
    F: FnMut() -> Result<(), AppError>,
{
    let settings = settings::load(paths)?;
    if !should_auto_check(&settings, now) {
        return Ok(false);
    }

    match runner() {
        Ok(()) => Ok(true),
        Err(error) if error.code == "updateAlreadyRunning" => Ok(false),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::updater::{
        errors,
        settings::StoredUpdateSettings,
        types::{CheckSourcePreference, DownloadSourcePreference, UpdateChannel},
    };
    use std::{
        fs,
        sync::atomic::{AtomicUsize, Ordering},
    };

    fn test_paths(name: &str) -> UpdatePaths {
        let root = std::env::temp_dir()
            .join("floral-notepaper-updater-tests")
            .join(name);
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale test dir");
        }
        UpdatePaths::new(root)
    }

    fn save_settings(
        paths: &UpdatePaths,
        auto_check: bool,
        last_auto_check_at: Option<DateTime<Utc>>,
        check_interval_hours: u32,
    ) {
        settings::save(
            paths,
            &StoredUpdateSettings {
                auto_check,
                auto_download: false,
                check_interval_hours,
                check_source_preference: CheckSourcePreference::GithubFirst,
                download_source_preference: DownloadSourcePreference::MirrorChyanFirst,
                channel: UpdateChannel::Stable,
                allow_prerelease: false,
                last_auto_check_at,
            },
        )
        .expect("save update settings");
    }

    #[test]
    fn does_not_trigger_when_auto_check_is_disabled() {
        let paths = test_paths("scheduler-disabled");
        save_settings(&paths, false, None, 24);
        let calls = AtomicUsize::new(0);

        let triggered = maybe_run_due_check(&paths, Utc::now(), || {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .expect("disabled auto check should not error");

        assert!(!triggered);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn triggers_immediately_when_no_previous_auto_check_exists() {
        let paths = test_paths("scheduler-first-run");
        save_settings(&paths, true, None, 24);
        let calls = AtomicUsize::new(0);

        let triggered = maybe_run_due_check(&paths, Utc::now(), || {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .expect("first auto check should not error");

        assert!(triggered);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn skips_when_interval_has_not_elapsed() {
        let paths = test_paths("scheduler-not-due");
        save_settings(
            &paths,
            true,
            Some(Utc::now() - ChronoDuration::hours(12)),
            24,
        );
        let calls = AtomicUsize::new(0);

        let triggered = maybe_run_due_check(&paths, Utc::now(), || {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .expect("not due auto check should not error");

        assert!(!triggered);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn triggers_once_when_interval_has_elapsed() {
        let paths = test_paths("scheduler-due");
        save_settings(
            &paths,
            true,
            Some(Utc::now() - ChronoDuration::hours(25)),
            24,
        );
        let calls = AtomicUsize::new(0);

        let triggered = maybe_run_due_check(&paths, Utc::now(), || {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .expect("due auto check should not error");

        assert!(triggered);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn skips_busy_iterations_without_overwriting_last_auto_check_at() {
        let paths = test_paths("scheduler-busy");
        let last_auto_check_at = Utc::now() - ChronoDuration::hours(48);
        save_settings(&paths, true, Some(last_auto_check_at), 24);

        let triggered = maybe_run_due_check(&paths, Utc::now(), || {
            Err(errors::app_error(
                "updateAlreadyRunning",
                "已有更新任务正在运行",
            ))
        })
        .expect("busy auto check should be ignored");

        assert!(!triggered);
        assert_eq!(
            settings::load(&paths)
                .expect("load saved settings")
                .last_auto_check_at,
            Some(last_auto_check_at)
        );
    }
}
