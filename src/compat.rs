//! C-compatible API using safe references instead of raw pointers.
//!
//! This module provides functions with names matching the C time API,
//! but using safe Rust references instead of raw pointers.

use crate::{ClockId, Timespec, Tm};
use std::ffi::CStr;
use std::io;

/// Get current time. Equivalent to `time(3)`.
///
/// If `tloc` is `Some`, the result is also stored there.
/// Returns the current time as seconds since the Unix epoch.
pub fn time(tloc: Option<&mut libc::time_t>) -> libc::time_t {
    let t = unsafe { libc::time(std::ptr::null_mut()) };
    if let Some(loc) = tloc {
        *loc = t;
    }
    t
}

/// Compute difference between two times. Equivalent to `difftime(3)`.
///
/// Returns `time1 - time0` as seconds.
pub fn difftime(time1: libc::time_t, time0: libc::time_t) -> f64 {
    unsafe { libc::difftime(time1, time0) }
}

/// Convert broken-down time to time since epoch. Equivalent to `mktime(3)`.
///
/// The `tm` structure is normalized as a side effect.
/// Returns `-1` on error.
pub fn mktime(tm: &mut Tm) -> libc::time_t {
    unsafe { libc::mktime(&mut tm.0) }
}

/// Convert time to local broken-down time. Equivalent to `localtime_r(3)`.
///
/// Returns `None` on error.
pub fn localtime<'a>(timep: &libc::time_t, result: &'a mut Tm) -> Option<&'a mut Tm> {
    let ptr = unsafe { libc::localtime_r(timep, &mut result.0) };
    if ptr.is_null() {
        None
    } else {
        Some(result)
    }
}

/// Convert time to UTC broken-down time. Equivalent to `gmtime_r(3)`.
///
/// Returns `None` on error.
pub fn gmtime<'a>(timep: &libc::time_t, result: &'a mut Tm) -> Option<&'a mut Tm> {
    let ptr = unsafe { libc::gmtime_r(timep, &mut result.0) };
    if ptr.is_null() {
        None
    } else {
        Some(result)
    }
}

/// Format time as a string. Equivalent to `strftime(3)`.
///
/// Writes the formatted string to `buf` and returns the number of bytes written
/// (excluding the null terminator). Returns `0` if the buffer is too small.
pub fn strftime(buf: &mut [u8], format: &CStr, tm: &Tm) -> usize {
    unsafe {
        libc::strftime(
            buf.as_mut_ptr() as *mut libc::c_char,
            buf.len(),
            format.as_ptr(),
            &tm.0,
        )
    }
}

/// Convert time to string. Equivalent to `ctime_r(3)`.
///
/// Writes to the provided 26-byte buffer and returns a reference to it on success.
pub fn ctime<'a>(timep: &libc::time_t, buf: &'a mut [i8; 26]) -> Option<&'a CStr> {
    let ptr = unsafe { libc::ctime_r(timep, buf.as_mut_ptr()) };
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(buf.as_ptr()) })
    }
}

/// Convert broken-down time to string. Equivalent to `asctime_r(3)`.
///
/// Writes to the provided 26-byte buffer and returns a reference to it on success.
pub fn asctime<'a>(tm: &Tm, buf: &'a mut [i8; 26]) -> Option<&'a CStr> {
    let ptr = unsafe { libc::asctime_r(&tm.0, buf.as_mut_ptr()) };
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(buf.as_ptr()) })
    }
}

/// Get time from a clock. Equivalent to `clock_gettime(2)`.
///
/// Stores the result in `tp` and returns `Ok(())` on success.
pub fn clock_gettime(clk_id: ClockId, tp: &mut Timespec) -> io::Result<()> {
    let mut ts: libc::timespec = unsafe { std::mem::zeroed() };
    let ret = unsafe { libc::clock_gettime(clk_id as libc::clockid_t, &mut ts) };
    if ret == 0 {
        tp.sec = ts.tv_sec;
        tp.nsec = ts.tv_nsec;
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Get clock resolution. Equivalent to `clock_getres(2)`.
///
/// Stores the resolution in `res` and returns `Ok(())` on success.
pub fn clock_getres(clk_id: ClockId, res: &mut Timespec) -> io::Result<()> {
    let mut ts: libc::timespec = unsafe { std::mem::zeroed() };
    let ret = unsafe { libc::clock_getres(clk_id as libc::clockid_t, &mut ts) };
    if ret == 0 {
        res.sec = ts.tv_sec;
        res.nsec = ts.tv_nsec;
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Suspend execution for an interval. Equivalent to `nanosleep(2)`.
///
/// If interrupted, the remaining time is stored in `rem` (if provided).
pub fn nanosleep(req: &Timespec, rem: Option<&mut Timespec>) -> io::Result<()> {
    let req_ts = libc::timespec {
        tv_sec: req.sec,
        tv_nsec: req.nsec,
    };
    let mut rem_ts: libc::timespec = unsafe { std::mem::zeroed() };
    let rem_ptr = if rem.is_some() {
        &mut rem_ts as *mut _
    } else {
        std::ptr::null_mut()
    };

    let ret = unsafe { libc::nanosleep(&req_ts, rem_ptr) };
    if ret == 0 {
        Ok(())
    } else {
        if let Some(r) = rem {
            r.sec = rem_ts.tv_sec;
            r.nsec = rem_ts.tv_nsec;
        }
        Err(io::Error::last_os_error())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_time() {
        let t1 = time(None);
        let mut t2: libc::time_t = 0;
        let t3 = time(Some(&mut t2));
        assert!(t1 > 0);
        assert_eq!(t2, t3);
        assert!(t3 >= t1);
    }

    #[test]
    fn test_difftime() {
        let diff = difftime(1100, 1000);
        assert!((diff - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_localtime_gmtime() {
        let t = time(None);
        let mut local = Tm::new(0, 0, 0, 0, 0, 0);
        let mut utc = Tm::new(0, 0, 0, 0, 0, 0);

        assert!(localtime(&t, &mut local).is_some());
        assert!(gmtime(&t, &mut utc).is_some());

        assert!(local.calendar_year() >= 2024);
        assert!(utc.calendar_year() >= 2024);
    }

    #[test]
    fn test_mktime_roundtrip() {
        let t1 = time(None);
        let mut tm = Tm::new(0, 0, 0, 0, 0, 0);
        localtime(&t1, &mut tm);
        let t2 = mktime(&mut tm);
        assert_eq!(t1, t2);
    }

    #[test]
    fn test_strftime() {
        let t = time(None);
        let mut tm = Tm::new(0, 0, 0, 0, 0, 0);
        localtime(&t, &mut tm);

        let fmt = CString::new("%Y-%m-%d").unwrap();
        let mut buf = [0u8; 32];
        let len = strftime(&mut buf, &fmt, &tm);
        assert!(len > 0);

        let s = std::str::from_utf8(&buf[..len]).unwrap();
        assert!(s.starts_with("20")); // Year starts with 20xx
    }

    #[test]
    fn test_ctime_asctime() {
        let t = time(None);
        let mut buf = [0i8; 26];
        let result = ctime(&t, &mut buf);
        assert!(result.is_some());

        let mut tm = Tm::new(0, 0, 0, 0, 0, 0);
        localtime(&t, &mut tm);
        let mut buf2 = [0i8; 26];
        let result2 = asctime(&tm, &mut buf2);
        assert!(result2.is_some());
    }

    #[test]
    fn test_clock_gettime() {
        let mut ts = Timespec { sec: 0, nsec: 0 };
        assert!(clock_gettime(ClockId::Realtime, &mut ts).is_ok());
        assert!(ts.sec > 0);
    }

    #[test]
    fn test_clock_getres() {
        let mut res = Timespec { sec: 0, nsec: 0 };
        assert!(clock_getres(ClockId::Realtime, &mut res).is_ok());
        // Resolution should be small (typically 1ns or 1us)
        assert!(res.sec == 0 || res.nsec > 0);
    }

    #[test]
    fn test_nanosleep() {
        let req = Timespec { sec: 0, nsec: 1000 }; // 1 microsecond
        assert!(nanosleep(&req, None).is_ok());
    }
}
