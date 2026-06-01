//! Process-group teardown reaches a *grandchild* (unix-only).
//!
//! This is the integration proof for the process-group leak fix. The old
//! supervisor relied on tokio's `kill_on_drop(true)`, which only ever targets
//! the *direct* child: a grandchild the child forked is left reparented to
//! init and leaks. The fix spawns every child in its OWN process group
//! (`Command::process_group(0)`, mirrored here) and tears the whole group down
//! with `killpg` ([`reaper::signal_group`]). These tests stand up a real
//! child→grandchild process tree and assert the grandchild dies when, and only
//! when, the *group* is signalled.
//!
//! Two levels of proof:
//!  - `killpg_reaches_grandchild` — the central claim: a forked grandchild in
//!    the child's group is killed by a single `signal_group(SIGKILL)`, which a
//!    per-child `kill()` would NOT have done.
//!  - `reap_stale_groups_kills_a_real_live_group` — the boot-reaper round trip:
//!    `write_pidfile` a live group, `reap_stale_groups`, assert it was recorded
//!    as alive and SIGKILLed and the whole group (grandchild included) dies.
//!
//! `nix` (a `cfg(unix)` lib dep of the crate) is re-declared as a unix-only
//! dev-dep so the test can signal/probe groups directly, exactly as `reaper.rs`
//! does internally.

#![cfg(unix)]

use std::io::{BufRead, BufReader};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use nix::errno::Errno;
use nix::sys::signal::Signal;
use nix::unistd::Pid;
use starter_ext_supervisor::{reap_stale_groups, reaper};

/// `true` while pid `pid` exists, via `kill(pid, 0)` (signal 0 probes liveness
/// without delivering anything). `EPERM` counts as alive (exists, not ours).
fn pid_alive(pid: i32) -> bool {
    matches!(
        nix::sys::signal::kill(Pid::from_raw(pid), None),
        Ok(()) | Err(Errno::EPERM)
    )
}

/// The process group a pid currently belongs to (its pgid), via `getpgid`.
fn pgid_of(pid: i32) -> Option<i32> {
    nix::unistd::getpgid(Some(Pid::from_raw(pid)))
        .ok()
        .map(|p| p.as_raw())
}

/// Poll up to `timeout` for `pid` to become dead.
fn wait_until_dead(pid: i32, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if !pid_alive(pid) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    !pid_alive(pid)
}

/// SIGKILL the whole group then reap the (now-zombie) shell. Best-effort.
fn cleanup(mut child: Child, pgid: i32) {
    let _ = reaper::signal_group(pgid, Signal::SIGKILL);
    let _ = child.kill();
    let _ = child.wait();
}

/// Spawn a group leader (`/bin/sh`) that itself forks a long-sleeping
/// *grandchild* into the same process group, then prints the grandchild's pid
/// on stdout and sleeps. Returns `(child_handle, pgid, grandchild_pid)`.
///
/// This is the exact shape the leak fix targets: the supervisor's direct child
/// is the shell; the grandchild is the worker the shell forked. `kill_on_drop`
/// would reach only the shell.
fn spawn_child_with_grandchild() -> (Child, i32, i32) {
    let mut cmd = Command::new("/bin/sh");
    // `sleep 300 &` forks the grandchild (inheriting the shell's process
    // group); `echo $!` reports its pid; the final `sleep` keeps the leader
    // alive so the group exists while we assert.
    cmd.arg("-c").arg("sleep 300 & echo $!; sleep 300");
    cmd.stdout(Stdio::piped());
    // Mirror the supervisor: the leader heads its own process group, so the
    // group id equals the leader pid and one killpg reaches the grandchild.
    cmd.process_group(0);
    let mut child = cmd.spawn().expect("spawn /bin/sh group leader");

    let pgid = child.id() as i32;

    // Read the single line the shell prints: the grandchild's pid.
    let stdout = child.stdout.take().expect("piped stdout");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).expect("read grandchild pid line");
    let grandchild_pid: i32 = line.trim().parse().expect("grandchild pid is an integer");

    (child, pgid, grandchild_pid)
}

/// THE central proof: a grandchild forked by the child shares the child's
/// process group, and a single `killpg` (`reaper::signal_group`) on that group
/// kills the grandchild — which a per-child `kill()` alone never would.
#[test]
fn killpg_reaches_grandchild() {
    let (child, pgid, grandchild) = spawn_child_with_grandchild();

    // Guard: whatever happens below, never leak the test's own processes.
    struct Guard(Option<Child>, i32);
    impl Drop for Guard {
        fn drop(&mut self) {
            if let Some(c) = self.0.take() {
                cleanup(c, self.1);
            }
        }
    }
    let mut guard = Guard(Some(child), pgid);

    // 1. Both the leader and the grandchild are alive.
    assert!(pid_alive(pgid), "group leader must be alive");
    assert!(pid_alive(grandchild), "grandchild must be alive");
    assert_ne!(pgid, grandchild, "grandchild must be a distinct process");

    // 2. The grandchild is in the leader's process group — so a group kill
    //    reaches it. (This is the invariant the fix depends on.)
    assert_eq!(
        pgid_of(grandchild),
        Some(pgid),
        "grandchild must belong to the leader's process group {pgid}"
    );
    assert!(
        reaper::group_alive(pgid),
        "reaper::group_alive must agree the group is live"
    );

    // 3. Tear the GROUP down — the killpg the supervisor performs on teardown.
    //    A `kill(child)` here would orphan the grandchild; killpg does not.
    assert!(
        reaper::signal_group(pgid, Signal::SIGKILL),
        "signal_group must report the live group was signalled"
    );

    // 4. The grandchild specifically dies — the leak the fix prevents. The
    //    grandchild was reparented to init (the test process is not its
    //    parent), so once it exits it is reaped automatically and `kill(0)`
    //    reports it gone.
    assert!(
        wait_until_dead(grandchild, Duration::from_secs(3)),
        "grandchild {grandchild} must die when its process group is killpg'd \
         (kill_on_drop on the direct child alone would have leaked it)"
    );

    // 5. The leader dies too. The test process IS the leader's parent, so we
    //    must `wait()` to reap it — otherwise it lingers as a zombie and a
    //    bare `kill(pid, 0)` would still report it "alive".
    if let Some(mut c) = guard.0.take() {
        let status = c
            .wait()
            .expect("wait on the killpg'd group leader");
        assert!(
            !status.success(),
            "leader must have been killed by the group signal, not exit cleanly"
        );
    }
    assert!(
        wait_until_dead(pgid, Duration::from_secs(3)),
        "group leader {pgid} must be gone after being reaped"
    );
}

/// Boot-reaper round trip against a real live group (leader + grandchild):
/// `write_pidfile` for the live group, `reap_stale_groups`, assert it recorded
/// the group as alive, SIGKILLed it, consumed the pidfile, and the whole group
/// — grandchild included — is gone.
#[test]
fn reap_stale_groups_kills_a_real_live_group() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (child, pgid, grandchild) = spawn_child_with_grandchild();

    struct Guard(Option<Child>, i32);
    impl Drop for Guard {
        fn drop(&mut self) {
            if let Some(c) = self.0.take() {
                cleanup(c, self.1);
            }
        }
    }
    let mut guard = Guard(Some(child), pgid);

    assert!(pid_alive(pgid), "leader alive before reaping");
    assert!(pid_alive(grandchild), "grandchild alive before reaping");
    assert!(reaper::group_alive(pgid), "group alive before reaping");

    // Record the live group exactly as the supervisor does on spawn.
    reaper::write_pidfile(dir.path(), "com.acme.leaky", pgid).expect("write_pidfile");
    let pidfile = dir.path().join("com.acme.leaky.pid");
    assert!(pidfile.exists(), "pidfile must exist after write_pidfile");

    // The code under test: a boot-time reap pass.
    let report = reap_stale_groups(dir.path());

    assert_eq!(report.total(), 1, "one pidfile processed: {report:#?}");
    assert_eq!(report.killed(), 1, "the live group must be killed: {report:#?}");
    assert_eq!(report.groups.len(), 1);
    let g = &report.groups[0];
    assert_eq!(g.extension_id, "com.acme.leaky");
    assert_eq!(g.pgid, pgid);
    assert!(g.was_alive, "group recorded as having been alive");

    // Pidfile is consumed by the reap pass.
    assert!(!pidfile.exists(), "pidfile must be removed after reaping");

    // The whole group dies — the forked grandchild (reparented to init, so
    // auto-reaped on exit) and the leader (our child, so we reap it).
    assert!(
        wait_until_dead(grandchild, Duration::from_secs(3)),
        "grandchild {grandchild} must die after reap_stale_groups SIGKILLed the group"
    );
    if let Some(mut c) = guard.0.take() {
        let status = c.wait().expect("wait on the reaped group leader");
        assert!(
            !status.success(),
            "leader must have been SIGKILLed, not exit cleanly"
        );
    }
    assert!(
        wait_until_dead(pgid, Duration::from_secs(3)),
        "group leader {pgid} must be gone after the reap + reaping its zombie"
    );
    assert!(
        !reaper::group_alive(pgid),
        "reaper::group_alive must report the group dead after reaping"
    );
}

/// A pidfile for an already-dead group is processed but not counted as killed
/// — the reaper distinguishes "we cleaned up a real leak" from "stale file for
/// a group that was already gone".
#[test]
fn reap_records_dead_group_as_not_killed() {
    let dir = tempfile::tempdir().expect("tempdir");
    // A pgid that is essentially never live.
    let dead_pgid = i32::MAX;
    assert!(!reaper::group_alive(dead_pgid), "precondition: group is dead");

    reaper::write_pidfile(dir.path(), "com.acme.gone", dead_pgid).expect("write_pidfile");
    let report = reap_stale_groups(dir.path());

    assert_eq!(report.total(), 1);
    assert_eq!(report.killed(), 0, "dead group is not a kill");
    assert!(!report.groups[0].was_alive);
}
