//! Parse a date from a Unix timestamp.
//! The implementation is adapted from musl.
//! 

use std::time::{SystemTime, UNIX_EPOCH};

// 2000-03-01 (mod 400 year, immediately after feb29)
const LEAPOCH: i64 = 946684800 + 86400 * (31 + 29);

const DAYS_PER_400Y: i64 = 365 * 400 + 97;
const DAYS_PER_100Y: i64 = 365 * 100 + 24;
const DAYS_PER_4Y: i64 = 365 * 4 + 1;

#[derive(Clone, Debug)]
pub struct Date {
    /// The number of seconds after the minute, normally in the range 0 to 59,
    /// but can be up to 60 to allow for leap seconds.
    pub seconds: i32,
    /// The number of minutes after the hour, in the range 0 to 59.
    pub minutes: i32,
    /// The number of hours past midnight, in the range 0 to 23.
    pub hours: i32,
    /// The day of the month, in the range 1 to 31.
    pub day_of_month: i32,
    /// The number of months since January, in the range 0 to 11.
    pub month: i32,
    /// The number of years since 1900.
    pub year: i32,
    /// The number of days since Sunday, in the range 0 to 6.
    pub day_of_week: i32,
    /// The number of days since January 1, in the range 0 to 365.
    pub day_of_year: i32
}

impl Date {
    #[inline]
    pub fn now() -> Option<Self> {
        Self::now_with_offset(0)
    }

    #[inline]
    pub fn now_with_offset(offset_hours: u8) -> Option<Self> {
        let seconds = offset_hours as u64 * 60 * 60;
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;

        Self::from_timestamp((timestamp.as_secs() + seconds) as i64)
    }

    // Adapted from:
    // https://git.musl-libc.org/cgit/musl/tree/src/time/__secs_to_tm.c?h=v1.2.4&id=f5f55d6589940fd2c2188d76686efe3a530e64e0
    pub fn from_timestamp(timestamp: i64) -> Option<Self> {
        const DAYS_IN_MONTH: [u8; 12] = [31, 30, 31, 30, 31, 31, 30, 31, 30, 31, 31, 29];

        // Reject values whose year would overflow an int
        if timestamp < i32::MIN as i64 * 31622400 ||
            timestamp > i32::MAX as i64 * 31622400
        {
            return None;
        }

        let secs = timestamp - LEAPOCH;
        let mut days = secs / 86400;
        let mut remsecs = secs % 86400;

        if remsecs < 0 {
            remsecs += 86400;
            days -= 1;
        }

        let mut wday = (3 + days) % 7;
        if wday < 0 {
            wday += 7;
        }

        let mut qc_cycles = days / DAYS_PER_400Y;
        let mut remdays = days % DAYS_PER_400Y;

        if remdays < 0 {
            remdays += DAYS_PER_400Y;
            qc_cycles -= 1;
        }

        let mut c_cycles = remdays / DAYS_PER_100Y;
        if c_cycles == 4 {
            c_cycles -= 1;
        }

        remdays -= c_cycles * DAYS_PER_100Y;

        let mut q_cycles = remdays / DAYS_PER_4Y;
        if q_cycles == 25 {
            q_cycles -= 1;
        }

        remdays -= q_cycles * DAYS_PER_4Y;

        let mut remyears = remdays / 365;
        if remyears == 4 {
            remyears -= 1;
        }

        remdays -= remyears * 365;

        let leap = if remyears == 0 && (q_cycles != 0 || c_cycles == 0) {
            1
        } else {
            0
        };

        let mut yday = remdays + 31 + 28 + leap;
        if yday >= 365 + leap {
            yday -= 365 + leap;
        }

        let mut years = remyears + 4 * q_cycles + 100 * c_cycles + 400 * qc_cycles;

        let mut months = 0;
        while DAYS_IN_MONTH[months as usize] as i64 <= remdays {
            remdays -= DAYS_IN_MONTH[months as usize] as i64;
            months += 1;
        }

        if months >= 10 {
            months -= 12;
            years += 1;
        }

        if years + 100 > i32::MAX as i64 || years + 100 < i32::MIN as i64 {
		    return None;
        }

        let remsecs = remsecs as i32;

        Some(Self {
            seconds: remsecs % 60,
            minutes: remsecs / 60 % 60,
            hours: remsecs / 3600,
            day_of_month: remdays as i32 + 1,
            month: months as i32 + 2,
            year: years as i32 + 100,
            day_of_week: wday as i32,
            day_of_year: yday as i32
        })
    }

    #[inline]
    pub fn current_year(&self) -> i32 {
        1900 + self.year
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correctly_parses_dates() {
        let date = Date::from_timestamp(1652978610).unwrap();
        assert_eq!(date.year, 122);
        assert_eq!(date.current_year(), 2022);
        assert_eq!(date.month, 4);
        assert_eq!(date.day_of_month, 19);
        assert_eq!(date.hours, 16);
        assert_eq!(date.minutes, 43);
        assert_eq!(date.seconds, 30);
        assert_eq!(date.day_of_week, 4);
        assert_eq!(date.day_of_year, 138);
    }
}
