//! Display timing settings, shared by the display and control applications.
//!
//! Both live in the generic SETTINGS key/value table so they survive restarts.
//! All range enforcement lives here so the control application's sliders and the
//! display application's playback can never disagree about what is valid.

use crate::backend::database::sql::SQL;
use crate::error::Res;

/// Seconds a photo stays on screen before the next one is loaded.
pub const PERIOD_MIN: f32 = 0.1;
pub const PERIOD_MAX: f32 = 120.0;
pub const PERIOD_DEFAULT: f32 = 10.0;

/// Seconds spent crossfading from the outgoing photo to the incoming one.
pub const BLUR_MIN: f32 = 0.1;
pub const BLUR_MAX: f32 = 5.0;
pub const BLUR_DEFAULT: f32 = 1.0;

pub const PERIOD_KEY: &str = "period";
pub const BLUR_DURATION_KEY: &str = "blur_duration";

/// The largest crossfade permitted for a given period.
///
/// The crossfade has to finish before the next photo is due, so it is capped by
/// the period as well as by [`BLUR_MAX`]. At the very bottom of the period range
/// the two bounds meet (period 0.1 allows exactly 0.1), which degenerates into a
/// continuous crossfade rather than an invalid state.
pub fn max_blur_for(period: f32) -> f32 {
    BLUR_MAX.min(period).max(BLUR_MIN)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub period: f32,
    pub blur_duration: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            period: PERIOD_DEFAULT,
            blur_duration: BLUR_DEFAULT,
        }
    }
}

impl Settings {
    /// Build settings with both values forced into their valid ranges, including
    /// the crossfade's dependency on the period.
    pub fn clamped(period: f32, blur_duration: f32) -> Self {
        // A NaN from a corrupt row or a malformed packet would poison every
        // later comparison, so fall back to the defaults rather than propagate.
        let period = if period.is_finite() { period } else { PERIOD_DEFAULT };
        let blur_duration = if blur_duration.is_finite() { blur_duration } else { BLUR_DEFAULT };

        let period = period.clamp(PERIOD_MIN, PERIOD_MAX);
        let blur_duration = blur_duration.clamp(BLUR_MIN, max_blur_for(period));

        Self { period, blur_duration }
    }

    /// Read both settings, falling back to the default for any that is missing
    /// or unparseable — a fresh install has neither row.
    pub async fn load() -> Res<Self> {
        let period = SQL::select_setting_by_name(PERIOD_KEY)
            .await?
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(PERIOD_DEFAULT);

        let blur_duration = SQL::select_setting_by_name(BLUR_DURATION_KEY)
            .await?
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(BLUR_DEFAULT);

        Ok(Self::clamped(period, blur_duration))
    }

    /// Persist both settings, clamping first so an out-of-range value can never
    /// reach the database.
    pub async fn save(&self) -> Res<Self> {
        let clamped = Self::clamped(self.period, self.blur_duration);

        SQL::insert_or_update_setting(PERIOD_KEY, &clamped.period.to_string()).await?;
        SQL::insert_or_update_setting(
            BLUR_DURATION_KEY,
            &clamped.blur_duration.to_string(),
        ).await?;

        Ok(clamped)
    }

    pub fn period_duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs_f32(self.period)
    }

    pub fn blur_duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs_f32(self.blur_duration)
    }
}
