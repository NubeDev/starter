//! Read the current process resident set size from `/proc/self/statm`.
//!
//! The soak's memory bound is "no unbounded growth", which needs a live RSS
//! reading without a heavyweight metrics dependency. On Linux `/proc/self/statm`
//! reports the resident page count in its second field; multiplying by the page
//! size gives bytes. Non-Linux hosts return `None`, so the soak's RSS assertions
//! are skipped there rather than failing on a platform that has no procfs.

/// Current resident set size in bytes, or `None` if it cannot be read (non-Linux
/// or an unreadable procfs).
pub fn resident_bytes() -> Option<u64> {
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let resident_pages: u64 = statm.split_whitespace().nth(1)?.parse().ok()?;
    Some(resident_pages * page_size())
}

/// The system page size in bytes (4 KiB on the platforms the soak runs on).
fn page_size() -> u64 {
    let sz = libc_sysconf_pagesize();
    if sz > 0 {
        sz as u64
    } else {
        4096
    }
}

#[cfg(target_os = "linux")]
fn libc_sysconf_pagesize() -> i64 {
    // Avoid a libc dependency: read the page size via the same procfs-adjacent
    // constant the kernel exposes. `getconf PAGESIZE` is 4096 on every target the
    // soak runs on; reading it from sysconf keeps it correct on the rare huge-page
    // host without pulling a new crate.
    extern "C" {
        fn sysconf(name: i32) -> i64;
    }
    // _SC_PAGESIZE is 30 on Linux/glibc and musl.
    const SC_PAGESIZE: i32 = 30;
    unsafe { sysconf(SC_PAGESIZE) }
}

#[cfg(not(target_os = "linux"))]
fn libc_sysconf_pagesize() -> i64 {
    4096
}
