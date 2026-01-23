//! More-idiomatic wrapper around the libc time API.

use std::ffi::CStr;
use std::fmt;
use std::io;

pub mod compat;

/// Represents a point in time as seconds since the Unix epoch (1970-01-01 00:00:00 UTC).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Time(libc::time_t);

impl Time {
    /// Returns the current time.
    pub fn now() -> Self {
        let t = unsafe { libc::time(std::ptr::null_mut()) };
        Time(t)
    }

    /// Creates a `Time` from a raw `time_t` value.
    pub fn from_raw(t: libc::time_t) -> Self {
        Time(t)
    }

    /// Returns the raw `time_t` value.
    pub fn as_raw(&self) -> libc::time_t {
        self.0
    }

    /// Returns the difference in seconds between `self` and `other`.
    pub fn diff(&self, other: &Time) -> f64 {
        unsafe { libc::difftime(self.0, other.0) }
    }

    /// Converts to a `Tm` in the local timezone.
    pub fn to_local(&self) -> Option<Tm> {
        let mut result: libc::tm = unsafe { std::mem::zeroed() };
        let ptr = unsafe { libc::localtime_r(&self.0, &mut result) };
        if ptr.is_null() {
            None
        } else {
            Some(Tm(result))
        }
    }

    /// Converts to a `Tm` in UTC.
    pub fn to_utc(&self) -> Option<Tm> {
        let mut result: libc::tm = unsafe { std::mem::zeroed() };
        let ptr = unsafe { libc::gmtime_r(&self.0, &mut result) };
        if ptr.is_null() {
            None
        } else {
            Some(Tm(result))
        }
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(tm) = self.to_local() {
            write!(f, "{}", tm)
        } else {
            write!(f, "Time({})", self.0)
        }
    }
}

/// Broken-down time representation (similar to `struct tm`).
#[derive(Debug, Clone)]
pub struct Tm(pub(crate) libc::tm);

impl Tm {
    /// Creates a new `Tm` with the specified components.
    ///
    /// - `year`: years since 1900
    /// - `month`: month (0-11)
    /// - `day`: day of month (1-31)
    /// - `hour`: hour (0-23)
    /// - `minute`: minute (0-59)
    /// - `second`: second (0-60, 60 for leap second)
    pub fn new(year: i32, month: i32, day: i32, hour: i32, minute: i32, second: i32) -> Self {
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        tm.tm_year = year;
        tm.tm_mon = month;
        tm.tm_mday = day;
        tm.tm_hour = hour;
        tm.tm_min = minute;
        tm.tm_sec = second;
        tm.tm_isdst = -1; // Let mktime determine DST
        Tm(tm)
    }

    /// Creates a `Tm` from a calendar date and time.
    ///
    /// - `year`: actual year (e.g., 2024)
    /// - `month`: month (1-12)
    /// - `day`: day of month (1-31)
    /// - `hour`: hour (0-23)
    /// - `minute`: minute (0-59)
    /// - `second`: second (0-59)
    pub fn from_date(
        year: i32,
        month: i32,
        day: i32,
        hour: i32,
        minute: i32,
        second: i32,
    ) -> Self {
        Self::new(year - 1900, month - 1, day, hour, minute, second)
    }

    /// Returns the second (0-60).
    pub fn second(&self) -> i32 {
        self.0.tm_sec
    }

    /// Returns the minute (0-59).
    pub fn minute(&self) -> i32 {
        self.0.tm_min
    }

    /// Returns the hour (0-23).
    pub fn hour(&self) -> i32 {
        self.0.tm_hour
    }

    /// Returns the day of month (1-31).
    pub fn day(&self) -> i32 {
        self.0.tm_mday
    }

    /// Returns the month (0-11).
    pub fn month(&self) -> i32 {
        self.0.tm_mon
    }

    /// Returns the year since 1900.
    pub fn year(&self) -> i32 {
        self.0.tm_year
    }

    /// Returns the actual calendar year.
    pub fn calendar_year(&self) -> i32 {
        self.0.tm_year + 1900
    }

    /// Returns the calendar month (1-12).
    pub fn calendar_month(&self) -> i32 {
        self.0.tm_mon + 1
    }

    /// Returns the day of week (0 = Sunday, 6 = Saturday).
    pub fn weekday(&self) -> i32 {
        self.0.tm_wday
    }

    /// Returns the day of year (0-365).
    pub fn year_day(&self) -> i32 {
        self.0.tm_yday
    }

    /// Returns whether DST is in effect.
    /// - `Some(true)`: DST is in effect
    /// - `Some(false)`: DST is not in effect
    /// - `None`: DST information unavailable
    pub fn is_dst(&self) -> Option<bool> {
        match self.0.tm_isdst {
            x if x > 0 => Some(true),
            0 => Some(false),
            _ => None,
        }
    }

    /// Converts this broken-down time to a `Time` (seconds since epoch).
    /// This interprets the time as local time.
    pub fn to_time(&mut self) -> Option<Time> {
        let result = unsafe { libc::mktime(&mut self.0) };
        if result == -1 {
            None
        } else {
            Some(Time(result))
        }
    }

    /// Formats the time according to the given format string.
    /// Uses the same format specifiers as `strftime(3)`.
    pub fn format(&self, fmt: &str) -> io::Result<String> {
        let fmt_cstr = std::ffi::CString::new(fmt)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        let mut buf = vec![0u8; 256];
        loop {
            let len = unsafe {
                libc::strftime(
                    buf.as_mut_ptr() as *mut libc::c_char,
                    buf.len(),
                    fmt_cstr.as_ptr(),
                    &self.0,
                )
            };

            if len > 0 {
                buf.truncate(len);
                return String::from_utf8(buf)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e));
            } else if buf.len() >= 4096 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "format string produces too large output",
                ));
            } else {
                buf.resize(buf.len() * 2, 0);
            }
        }
    }
}

impl fmt::Display for Tm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.format("%Y-%m-%d %H:%M:%S") {
            Ok(s) => write!(f, "{}", s),
            Err(_) => write!(
                f,
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                self.calendar_year(),
                self.calendar_month(),
                self.day(),
                self.hour(),
                self.minute(),
                self.second()
            ),
        }
    }
}

/// Returns the current time as a formatted string using `ctime`.
/// The returned string includes a trailing newline.
pub fn ctime(time: &Time) -> Option<String> {
    let mut buf = [0i8; 26];
    let ptr = unsafe { libc::ctime_r(&time.0, buf.as_mut_ptr()) };
    if ptr.is_null() {
        None
    } else {
        let cstr = unsafe { CStr::from_ptr(buf.as_ptr()) };
        cstr.to_str().ok().map(|s| s.to_owned())
    }
}

/// Returns the current time as a formatted string using `asctime`.
/// The returned string includes a trailing newline.
pub fn asctime(tm: &Tm) -> Option<String> {
    let mut buf = [0i8; 26];
    let ptr = unsafe { libc::asctime_r(&tm.0, buf.as_mut_ptr()) };
    if ptr.is_null() {
        None
    } else {
        let cstr = unsafe { CStr::from_ptr(buf.as_ptr()) };
        cstr.to_str().ok().map(|s| s.to_owned())
    }
}

/// Clock types for `clock_gettime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ClockId {
    /// System-wide realtime clock.
    Realtime = libc::CLOCK_REALTIME,
    /// Monotonic clock that cannot be set.
    Monotonic = libc::CLOCK_MONOTONIC,
    /// High-resolution per-process timer.
    ProcessCputime = libc::CLOCK_PROCESS_CPUTIME_ID,
    /// Thread-specific CPU-time clock.
    ThreadCputime = libc::CLOCK_THREAD_CPUTIME_ID,
}

/// High-resolution time specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timespec {
    /// Seconds.
    pub sec: i64,
    /// Nanoseconds (0 to 999,999,999).
    pub nsec: i64,
}

impl Timespec {
    /// Gets the current time from the specified clock.
    pub fn now(clock: ClockId) -> io::Result<Self> {
        let mut ts: libc::timespec = unsafe { std::mem::zeroed() };
        let ret = unsafe { libc::clock_gettime(clock as libc::clockid_t, &mut ts) };
        if ret == 0 {
            Ok(Timespec {
                sec: ts.tv_sec,
                nsec: ts.tv_nsec,
            })
        } else {
            Err(io::Error::last_os_error())
        }
    }

    /// Returns the time as floating-point seconds.
    pub fn as_secs_f64(&self) -> f64 {
        self.sec as f64 + self.nsec as f64 / 1_000_000_000.0
    }

    /// Returns the total nanoseconds.
    pub fn as_nanos(&self) -> i128 {
        self.sec as i128 * 1_000_000_000 + self.nsec as i128
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_now() {
        let t1 = Time::now();
        let t2 = Time::now();
        assert!(t2.as_raw() >= t1.as_raw());
    }

    #[test]
    fn test_time_diff() {
        let t1 = Time::from_raw(1000);
        let t2 = Time::from_raw(1100);
        assert!((t2.diff(&t1) - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_tm_components() {
        let tm = Tm::from_date(2024, 6, 15, 10, 30, 45);
        assert_eq!(tm.calendar_year(), 2024);
        assert_eq!(tm.calendar_month(), 6);
        assert_eq!(tm.day(), 15);
        assert_eq!(tm.hour(), 10);
        assert_eq!(tm.minute(), 30);
        assert_eq!(tm.second(), 45);
    }

    #[test]
    fn test_tm_format() {
        let tm = Tm::from_date(2024, 6, 15, 10, 30, 45);
        let formatted = tm.format("%Y-%m-%d").unwrap();
        assert_eq!(formatted, "2024-06-15");
    }

    #[test]
    fn test_to_local_and_back() {
        let now = Time::now();
        let mut local = now.to_local().unwrap();
        let back = local.to_time().unwrap();
        assert_eq!(now.as_raw(), back.as_raw());
    }

    #[test]
    fn test_ctime_format() {
        let t = Time::now();
        let s = ctime(&t).unwrap();
        assert!(!s.is_empty());
        assert!(s.ends_with('\n'));
    }

    #[test]
    fn test_timespec_now() {
        let ts = Timespec::now(ClockId::Realtime).unwrap();
        assert!(ts.sec > 0);
        assert!(ts.nsec >= 0 && ts.nsec < 1_000_000_000);
    }

    #[test]
    fn test_timespec_monotonic() {
        let ts1 = Timespec::now(ClockId::Monotonic).unwrap();
        let ts2 = Timespec::now(ClockId::Monotonic).unwrap();
        assert!(ts2.as_nanos() >= ts1.as_nanos());
    }
}
