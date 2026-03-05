use bitflags::bitflags;
use std::slice::from_raw_parts;

use crate::bindings::*;

pub use crate::bindings::{
    amdsmi_clk_type_t as AmdClkType, amdsmi_memory_type_t as AmdMemoryType,
    amdsmi_status_t as AmdStatus, amdsmi_temperature_metric_t as AmdTemperatureMetric,
    amdsmi_temperature_type_t as AmdTemperatureType, amdsmi_voltage_metric_t as AmdVoltageMetric,
    amdsmi_voltage_type_t as AmdVoltageType,
};

bitflags! {
    /// List of all [`amdsmi_init_flags_t`] bitmask value to initialize AMD-SMI library for a given hardware type to analyze.
    #[derive(Debug, Clone, Copy)]
    pub struct InitFlags: amdsmi_init_flags_t {
        const ALL_PROCESSORS = amdsmi_init_flags_t_AMDSMI_INIT_ALL_PROCESSORS;
        const AMD_CPUS = amdsmi_init_flags_t_AMDSMI_INIT_AMD_CPUS;
        const AMD_GPUS = amdsmi_init_flags_t_AMDSMI_INIT_AMD_GPUS;
        const AMD_APUS = amdsmi_init_flags_t_AMDSMI_INIT_AMD_APUS;
        const NON_AMD_CPUS = amdsmi_init_flags_t_AMDSMI_INIT_NON_AMD_CPUS;
        const NON_AMD_GPUS = amdsmi_init_flags_t_AMDSMI_INIT_NON_AMD_GPUS;
    }
}

/// Parameters about energy consumption of a GPU.
#[derive(Debug, Default, Clone, Copy)]
pub struct AmdEnergyConsumption {
    /// The energy consumption value of an AMD GPU device since the last boot in micro Joules.
    pub energy: u64,
    /// Precision factor of the energy counter in micro Joules.
    pub resolution: f32,
    /// The time during which the energy value is recovered in ns.
    pub timestamp: u64,
}

/// Parameters about the engine activity usage: [`amdsmi_engine_usage_t`].
#[derive(Debug, Default, Clone, Copy)]
pub struct AmdEngineUsage {
    /// Main graphic core of AMD GPU, in percentage.
    pub gfx_activity: u32,
    /// Manage memory access and addresses translation, in percentage.
    pub mm_activity: u32,
    /// Memory controller managing access to VRAM in organizing writing/reading operations, in percentage.
    pub umc_activity: u32,
}

impl From<amdsmi_engine_usage_t> for AmdEngineUsage {
    fn from(info: amdsmi_engine_usage_t) -> Self {
        Self {
            gfx_activity: info.gfx_activity,
            mm_activity: info.mm_activity,
            umc_activity: info.umc_activity,
        }
    }
}

/// Parameters about power consumption: [`amdsmi_power_info_t`].
#[derive(Debug, Default, Clone, Copy)]
pub struct AmdPowerConsumption {
    /// Socket power in W.
    pub socket_power: u64,
    /// Current socket power in W, Mi 300+ Series cards.
    pub current_socket_power: u32,
    /// Average socket power in W, Navi + Mi 200 and earlier Series cards.
    pub average_socket_power: u32,
    /// GFX voltage measurement in mV.
    pub gfx_voltage: u64,
    /// SOC voltage measurement in mV.
    pub soc_voltage: u64,
    /// MEM voltage measurement in mV.
    pub mem_voltage: u64,
    /// The power limit in W.
    pub power_limit: u32,
}

impl From<amdsmi_power_info_t> for AmdPowerConsumption {
    fn from(info: amdsmi_power_info_t) -> Self {
        Self {
            socket_power: info.socket_power,
            current_socket_power: info.current_socket_power,
            average_socket_power: info.average_socket_power,
            gfx_voltage: info.gfx_voltage,
            soc_voltage: info.soc_voltage,
            mem_voltage: info.mem_voltage,
            power_limit: info.power_limit,
        }
    }
}

/// Parameters about engine activity usage by process: [`amdsmi_proc_info_t_memory_usage_`].
#[derive(Debug, Default, Clone, Copy)]
pub struct AmdProcessEngineUsage {
    /// Process graphic core unit usage in nanoseconds.
    pub gfx: u64,
    /// Encoding units usage in nanoseconds.
    pub enc: u64,
}

impl From<amdsmi_proc_info_t_engine_usage_> for AmdProcessEngineUsage {
    fn from(info: amdsmi_proc_info_t_engine_usage_) -> Self {
        Self {
            gfx: info.gfx,
            enc: info.enc,
        }
    }
}

/// Parameters about consumed memory by process: [`amdsmi_proc_info_t_memory_usage_`].
#[derive(Debug, Default, Clone)]
pub struct AmdProcessMemoryUsage {
    /// Process GTT memory usage in Bytes.
    pub gtt_mem: u64,
    /// Process CPU memory usage in Bytes.
    pub cpu_mem: u64,
    /// Process VRAM memory usage in Bytes.
    pub vram_mem: u64,
}

impl From<amdsmi_proc_info_t_memory_usage_> for AmdProcessMemoryUsage {
    fn from(info: amdsmi_proc_info_t_memory_usage_) -> Self {
        Self {
            gtt_mem: info.gtt_mem,
            cpu_mem: info.cpu_mem,
            vram_mem: info.vram_mem,
        }
    }
}

/// List of running process: [`amdsmi_proc_info_t`].
#[derive(Debug, Default, Clone)]
pub struct AmdProcess {
    /// ASCII path name of the process.
    pub name: String,
    /// process ID.
    pub pid: u32,
    /// Process memory usage in Bytes.
    pub mem: u64,
    pub engine_usage: AmdProcessEngineUsage,
    pub memory_usage: AmdProcessMemoryUsage,
    /// ASCII name of the process container.
    pub container_name: String,
    /// Number of compute units utilized.
    pub cu_occupancy: u32,
    /// Time that queues are evicted on a GPU in milliseconds.
    pub evicted_time: u32,
}

/// Allow to convert ASCII table to usable null terminated string.
fn c_buffer_to_string(buffer: &[i8]) -> String {
    let length = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    let bytes = &buffer[..length];
    String::from_utf8_lossy(unsafe { from_raw_parts(bytes.as_ptr() as *const u8, bytes.len()) })
        .into_owned()
}

impl From<amdsmi_proc_info_t> for AmdProcess {
    fn from(value: amdsmi_proc_info_t) -> Self {
        Self {
            name: c_buffer_to_string(&value.name),
            pid: value.pid,
            mem: value.mem,
            engine_usage: value.engine_usage.into(),
            memory_usage: value.memory_usage.into(),
            container_name: c_buffer_to_string(&value.container_name),
            cu_occupancy: value.cu_occupancy,
            evicted_time: value.evicted_time,
        }
    }
}

/// Parameters about [`amdsmi_clk_info_t`].
#[derive(Debug, Default, Clone)]
pub struct AmdClkInfo {
    /// Clock frequency in MHz.
    pub clk: u32,
    /// Minimal clock frequency in MHz.
    pub min_clk: u32,
    /// Maximal clock frequency in MHz.
    pub max_clk: u32,
    /// Clock locked status boolean status
    pub clk_locked: u8,
    /// Clock deep sleep status boolean status
    pub clk_deep_sleep: u8,
}

impl From<amdsmi_clk_info_t> for AmdClkInfo {
    fn from(value: amdsmi_clk_info_t) -> Self {
        Self {
            clk: value.clk,
            min_clk: value.min_clk,
            max_clk: value.max_clk,
            clk_locked: value.clk_locked,
            clk_deep_sleep: value.clk_deep_sleep,
        }
    }
}
