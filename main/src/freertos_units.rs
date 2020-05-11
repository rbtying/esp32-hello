//! Adapted from `freertos_rs`.
use esp_idf_sys::{configTICK_RATE_HZ, TickType_t};

#[allow(non_upper_case_globals)]
pub const portTICK_PERIOD_MS: u32 = 1000 / configTICK_RATE_HZ;

pub trait FreeRtosTimeUnits {
    fn get_tick_period_ms() -> u32;
    fn get_max_wait() -> u32;
}

#[derive(Copy, Clone, Default)]
pub struct FreeRtosTimeUnitsShimmed;
impl FreeRtosTimeUnits for FreeRtosTimeUnitsShimmed {
    fn get_tick_period_ms() -> u32 {
        portTICK_PERIOD_MS
    }
    fn get_max_wait() -> u32 {
        TickType_t::max_value()
    }
}

pub trait DurationTicks: Copy + Clone {
    /// Convert to ticks, the internal time measurement unit of FreeRTOS
    fn to_ticks(&self) -> TickType_t;
}

pub type Duration = DurationImpl<FreeRtosTimeUnitsShimmed>;

/// Time unit used by FreeRTOS, passed to the scheduler as ticks.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DurationImpl<T> {
    ticks: u32,
    _time_units: core::marker::PhantomData<T>,
}

impl<T> DurationImpl<T>
where
    T: FreeRtosTimeUnits + Copy,
{
    /// Milliseconds constructor
    pub fn ms(milliseconds: u32) -> Self {
        Self::ticks((milliseconds + T::get_tick_period_ms() - 1) / T::get_tick_period_ms())
    }

    pub fn ticks(ticks: u32) -> Self {
        DurationImpl {
            ticks,
            _time_units: core::marker::PhantomData,
        }
    }

    /// An infinite duration
    pub fn infinite() -> Self {
        Self::ticks(T::get_max_wait())
    }

    /// A duration of zero, for non-blocking calls
    pub fn zero() -> Self {
        Self::ticks(0)
    }

    /// Smallest unit of measurement, one tick
    pub fn eps() -> Self {
        Self::ticks(1)
    }

    pub fn to_ms(&self) -> u32 {
        self.ticks * T::get_tick_period_ms()
    }
}

impl<T> DurationTicks for DurationImpl<T>
where
    T: FreeRtosTimeUnits + Copy,
{
    fn to_ticks(&self) -> TickType_t {
        self.ticks
    }
}
