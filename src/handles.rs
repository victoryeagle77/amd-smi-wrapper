//! Handles used to manipulate the devices.

use std::{
    ffi::{CStr, c_char},
    mem::MaybeUninit,
    ptr,
    sync::Arc,
};

use crate::{
    AmdSmi,
    bindings::{
        AMDSMI_GPU_UUID_SIZE, AMDSMI_MAX_FAN_SPEED, amdsmi_clk_info_t, amdsmi_engine_usage_t,
        amdsmi_power_info_t, amdsmi_proc_info_t, amdsmi_processor_handle, amdsmi_socket_handle,
        amdsmi_status_t,
    },
    error::AmdError,
    metrics::*,
};

#[cfg(feature = "mock")]
use mockall::automock;

pub struct AmdSocketHandle {
    pub(crate) amdsmi: Arc<AmdSmi>,
    pub(crate) inner: amdsmi_socket_handle,
}

pub struct AmdProcessorHandle {
    pub(crate) amdsmi: Arc<AmdSmi>,
    pub(crate) inner: amdsmi_processor_handle,
}

/// Handle to a socket in the system.
#[cfg_attr(feature = "mock", automock(type ProcessorHandle=MockProcessorHandle;))]
pub trait SocketHandle {
    /// The type of processor handles returned by this socket.
    type ProcessorHandle: ProcessorHandle;

    /// Lists the processors associated to this socket.
    fn processor_handles(&self) -> Result<Vec<Self::ProcessorHandle>, AmdError>;
}

impl SocketHandle for AmdSocketHandle {
    type ProcessorHandle = AmdProcessorHandle;

    fn processor_handles(&self) -> Result<Vec<Self::ProcessorHandle>, AmdError> {
        let mut processor_count = 0;

        // Query the number of processor handles for the given socket.
        // SAFETY: According the AMD-SMI library documentation, passing `null_mut()` is safe which sets `processor_count` to the number of processors available for this socket.
        let result = unsafe {
            self.amdsmi.amdsmi.amdsmi_get_processor_handles(
                self.inner,
                &mut processor_count,
                ptr::null_mut(),
            )
        };
        self.amdsmi.check_status(result)?;

        // Allocate a vector of nulls.
        let mut processor_handles = vec![ptr::null_mut(); processor_count as usize];

        // Fill the buffer with processor handles.
        // SAFETY: `processor_handles.as_mut_ptr()` points to a memory block of sufficient size.
        //  According the AMD-SMI library documentation, the function writes at most `processor_count` handles ensuring no out-of-bounds access occurs.
        let result = unsafe {
            self.amdsmi.amdsmi.amdsmi_get_processor_handles(
                self.inner,
                &mut processor_count,
                processor_handles.as_mut_ptr(),
            )
        };

        self.amdsmi.check_status(result)?;

        processor_handles.truncate(processor_count as usize);
        Ok(processor_handles
            .into_iter()
            .map(|s| AmdProcessorHandle {
                amdsmi: Arc::clone(&self.amdsmi),
                inner: s,
            })
            .collect())
    }
}

/// Handle to a processor in a [socket](SocketHandle).
#[cfg_attr(feature = "mock", automock)]
pub trait ProcessorHandle {
    /// Retrieves a [`AmdEngineUsage`] structure containing all data about GPU device activities.
    fn device_activity(&self) -> Result<AmdEngineUsage, AmdError>;

    /// Retrieves a [`AmdClkInfo`] structure containing data about detected clock devices.
    ///
    /// # Arguments
    ///
    /// - `clk_type`: Clock devices existing among [`AmdClkType`] on hardware.
    fn device_clock_info(&self, clk_type: AmdClkType) -> Result<AmdClkInfo, AmdError>;

    /// Retrieves a [`AmdEnergyConsumption`] structure containing data about energy consumption of the GPU device.
    fn device_energy_consumption(&self) -> Result<AmdEnergyConsumption, AmdError>;

    /// Retrieves the fan speed ratio.
    fn device_fan_speed(&self, sensor_index: u32) -> Result<u32, AmdError>;

    /// Retrieves the memory consumption of the GPU device.
    ///
    /// # Arguments
    ///
    /// - `mem_type`: Memory devices existing among [`AmdMemoryType`] on hardware.
    fn device_memory_usage(&self, mem_type: AmdMemoryType) -> Result<u64, AmdError>;

    /// Retrieves the PCI bus traffic used by the GPU device.
    fn device_pci_usage(&self) -> Result<AmdPciTraffic, AmdError>;

    /// Retrieves a [`AmdPowerConsumption`] structure containing all data about GPU device power consumption.
    fn device_power_consumption(&self) -> Result<AmdPowerConsumption, AmdError>;
    /// Retrieves the power management status accessability of the GPU device.
    fn device_power_managment(&self) -> Result<bool, AmdError>;

    /// Retrieves a set of [`AmdProcess`] structure containing data about running processes on the GPU device.
    fn device_process_list(&self) -> Result<Vec<AmdProcess>, AmdError>;

    /// Retrieves the temperature of a given area of the GPU device.
    ///
    /// # Arguments
    ///
    /// - `sensor_type`: Thermal sensor [`AmdTemperatureType`] on AMD GPU hardware.
    /// - `metric`: Temperature type [`AmdTemperatureMetric`] analyzed (current, average...).
    fn device_temperature(
        &self,
        sensor_type: AmdTemperatureType,
        metric: AmdTemperatureMetric,
    ) -> Result<i64, AmdError>;

    /// Retrieves the UUID of the GPU device.
    fn device_uuid(&self) -> Result<String, AmdError>;

    /// Retrieves the voltage of a given area of the GPU device.
    ///
    /// # Arguments
    ///
    /// - `sensor_type`: Voltage retrieved by a [`AmdVoltageType`] sensor on AMD GPU hardware.
    /// - `metric`: Voltage type [`AmdVoltageMetric`] analyzed (current, average...).
    fn device_voltage(
        &self,
        sensor_type: AmdVoltageType,
        metric: AmdVoltageMetric,
    ) -> Result<i64, AmdError>;
}

impl ProcessorHandle for AmdProcessorHandle {
    fn device_activity(&self) -> Result<AmdEngineUsage, AmdError> {
        // Allocate uninitialized memory for the structure and avoid reading uninitialized memory before the FFI call.
        let mut info = MaybeUninit::<amdsmi_engine_usage_t>::uninit();

        // SAFETY: Pass a raw pointer to uninitialized memory to the FFI function.
        // According to AMD-SMI documentation, the function fully initializes the structure on success.
        // The `SUCCESS` return code `amdsmi_status_t` is checked before using the data.
        let result = unsafe {
            self.amdsmi
                .amdsmi
                .amdsmi_get_gpu_activity(self.inner, info.as_mut_ptr())
        };

        self.amdsmi.check_status(result)?;

        // SAFETY: `assume_init()` is safe because the FFI call succeeded and fully initialized `info`.
        let info = unsafe { info.assume_init() };
        Ok(info.into())
    }

    fn device_clock_info(&self, clk_type: AmdClkType) -> Result<AmdClkInfo, AmdError> {
        let mut info = MaybeUninit::<amdsmi_clk_info_t>::uninit();

        // SAFETY: Pass a pointer to uninitialized memory to the FFI function.
        // According to AMD-SMI documentation, the function fully initializes the `amdsmi_clk_info_t` on success.
        // The `SUCCESS` return code `amdsmi_status_t` is checked before using the data.
        let result = unsafe {
            self.amdsmi
                .amdsmi
                .amdsmi_get_clock_info(self.inner, clk_type, info.as_mut_ptr())
        };

        self.amdsmi.check_status(result)?;

        // SAFETY: `assume_init()` is safe because the FFI call succeeded and the structure was fully initialized by the library.
        let info = unsafe { info.assume_init() };
        Ok(info.into())
    }

    fn device_energy_consumption(&self) -> Result<AmdEnergyConsumption, AmdError> {
        let mut consumption = AmdEnergyConsumption {
            energy: 0,
            resolution: 0.0,
            timestamp: 0,
        };

        // SAFETY: Pass mutable pointers to the fields of `consumption` to the FFI function.
        // According to AMD-SMI documentation, the function writes all values on success and will not write beyond the memory locations provided.
        // The `SUCCESS` return code `amdsmi_status_t` is checked before using the data.
        let result = unsafe {
            self.amdsmi.amdsmi.amdsmi_get_energy_count(
                self.inner,
                &mut consumption.energy,
                &mut consumption.resolution,
                &mut consumption.timestamp,
            )
        };

        self.amdsmi.check_status(result)?;
        Ok(consumption)
    }

    fn device_fan_speed(&self, sensor_index: u32) -> Result<u32, AmdError> {
        let mut speed = 0;

        // SAFETY: Pass a mutable pointer to `speed` for the FFI function to write the current fan speed.
        // According to AMD-SMI documentation, the function writes a value between 0 and `AMDSMI_MAX_FAN_SPEED` to this pointer.
        // The `SUCCESS` return code `amdsmi_status_t` is checked before using the data.
        let result = unsafe {
            self.amdsmi
                .amdsmi
                .amdsmi_get_gpu_fan_speed(self.inner, sensor_index, &mut speed)
        };

        self.amdsmi.check_status(result)?;
        Ok((speed as u32 / AMDSMI_MAX_FAN_SPEED) * 100)
    }

    fn device_memory_usage(&self, mem_type: AmdMemoryType) -> Result<u64, AmdError> {
        let mut used = 0;

        // SAFETY: Pass a mutable pointer to `used` for the FFI function to write the memory usage.
        // According to AMD-SMI documentation, the function will write a valid value on success and will not write outside the provided memory location.
        // The `SUCCESS` return code `amdsmi_status_t` is checked before using the data.
        let result = unsafe {
            self.amdsmi
                .amdsmi
                .amdsmi_get_gpu_memory_usage(self.inner, mem_type, &mut used)
        };

        self.amdsmi.check_status(result)?;
        Ok(used)
    }

    fn device_pci_usage(&self) -> Result<AmdPciTraffic, AmdError> {
        let mut usage = AmdPciTraffic {
            sent: 0,
            received: 0,
            max_pkt_sz: 0,
        };

        // SAFETY: Pass mutable pointers to the fields of `usage` to the FFI function.
        // According to AMD-SMI documentation, the function writes all values on success or ignored them.
        // The `SUCCESS` return code `amdsmi_status_t` is checked before using the data.
        let result = unsafe {
            self.amdsmi.amdsmi.amdsmi_get_gpu_pci_throughput(
                self.inner,
                &mut usage.sent,
                &mut usage.received,
                &mut usage.max_pkt_sz,
            )
        };

        self.amdsmi.check_status(result)?;
        Ok(usage)
    }

    /// Retrieves a [`amdsmi_power_info_t`] structure containing all data about GPU device power consumption.
    fn device_power_consumption(&self) -> Result<AmdPowerConsumption, AmdError> {
        // Reserve uninitialized memory space for the C function to fill.
        let mut info = MaybeUninit::<amdsmi_power_info_t>::uninit();

        // SAFETY: Pass a raw pointer to uninitialized memory for the FFI function to write into.
        // `info` has exactly the size of `amdsmi_power_info_t`.
        // According to AMD-SMI documentation, the function fully initializes the structure on success.
        // The `SUCCESS` return code `amdsmi_status_t` is checked before using the data.
        let result = unsafe {
            self.amdsmi
                .amdsmi
                .amdsmi_get_power_info(self.inner, info.as_mut_ptr())
        };

        self.amdsmi.check_status(result)?;

        // SAFETY: `assume_init()` is safe because the FFI call returned SUCCESS, meaning `info` is fully initialized.
        let info = unsafe { info.assume_init() };
        Ok(info.into())
    }

    fn device_power_managment(&self) -> Result<bool, AmdError> {
        let mut enabled = false;

        // SAFETY: Pass a mutable pointer to `enabled` for the FFI function to write the power management status.
        // According to AMD-SMI documentation, the function will write a valid boolean value on success.
        // The `SUCCESS` return code `amdsmi_status_t` is checked before using the data.
        let result = unsafe {
            self.amdsmi
                .amdsmi
                .amdsmi_is_gpu_power_management_enabled(self.inner, &mut enabled)
        };

        self.amdsmi.check_status(result)?;
        Ok(enabled)
    }

    fn device_temperature(
        &self,
        sensor_type: AmdTemperatureType,
        metric: AmdTemperatureMetric,
    ) -> Result<i64, AmdError> {
        let mut temperature = 0;

        // SAFETY: Pass a mutable pointer to `temperature` for the FFI function to write the temperature value.
        // According to AMD-SMI documentation, the function writes the value to this pointer.
        // The `SUCCESS` return code `amdsmi_status_t` is checked before using the data.
        let result = unsafe {
            self.amdsmi.amdsmi.amdsmi_get_temp_metric(
                self.inner,
                sensor_type,
                metric,
                &mut temperature,
            )
        };

        self.amdsmi.check_status(result)?;
        Ok(temperature)
    }

    fn device_voltage(
        &self,
        sensor_type: AmdVoltageType,
        metric: AmdVoltageMetric,
    ) -> Result<i64, AmdError> {
        let mut voltage = 0;

        // SAFETY: Pass a non-null mutable pointer to `voltage` for the FFI function to write the voltage value.
        // According to AMD-SMI documentation, the function writes the value to this pointer.
        // The value is only read after confirming that the return status is SUCCESS.
        // The `SUCCESS` return code `amdsmi_status_t` is checked before using the data.
        let result = unsafe {
            self.amdsmi.amdsmi.amdsmi_get_gpu_volt_metric(
                self.inner,
                sensor_type,
                metric,
                &mut voltage,
            )
        };

        self.amdsmi.check_status(result)?;
        Ok(voltage)
    }

    fn device_process_list(&self) -> Result<Vec<AmdProcess>, AmdError> {
        let mut max_processes = 0;

        // SAFETY: Retrieves the total number of GPU processes.
        // Passing `null_mut()` as the buffer tells the FFI to only write the count to `max_processes`.
        // According to AMD-SMI documentation, `max_processes` will be updated with the actual number of processes.
        let result = unsafe {
            self.amdsmi.amdsmi.amdsmi_get_gpu_process_list(
                self.inner,
                &mut max_processes,
                ptr::null_mut(),
            )
        };

        match result {
            amdsmi_status_t::AMDSMI_STATUS_SUCCESS => {}
            amdsmi_status_t::AMDSMI_STATUS_OUT_OF_RESOURCES => {}
            err => {
                return Err(AmdError {
                    status: err,
                    message: None,
                });
            }
        }

        if max_processes == 0 {
            return Ok(Vec::new());
        }

        loop {
            let mut buffer: Vec<MaybeUninit<amdsmi_proc_info_t>> =
                Vec::with_capacity(max_processes as usize);

            let mut count = max_processes;

            // SAFETY: Pass a pointer to the uninitialized buffer.
            // According the AMD-SMI library documentation, all elements up to `count` are written in case of `SUCCESS` or `OUT_OF_RESOURCES`.
            // There is no uninitialized memory read before the function writes to it.
            let result = unsafe {
                self.amdsmi.amdsmi.amdsmi_get_gpu_process_list(
                    self.inner,
                    &mut count,
                    buffer.as_mut_ptr() as *mut amdsmi_proc_info_t,
                )
            };

            match result {
                // SAFETY: According to AMD-SMI documentation, all elements up to `count` are written to the provided buffer.
                // Allocated `max_processes` elements in `SUCCESS` status implies all elements are initialized.
                amdsmi_status_t::AMDSMI_STATUS_SUCCESS => unsafe {
                    buffer.set_len(count as usize);
                    let processes = buffer
                        .into_iter()
                        .map(|x| AmdProcess::from(x.assume_init()))
                        .collect();

                    return Ok(processes);
                },
                // According to AMD-SMI documentation: The buffer was filled up to its capacity.
                // A counter is used to contain the actual total number of processes.
                // If The buffer was too small, we retry with the new required size.
                amdsmi_status_t::AMDSMI_STATUS_OUT_OF_RESOURCES => {
                    max_processes = count;
                    continue;
                }
                err => {
                    return Err(AmdError {
                        status: err,
                        message: None,
                    });
                }
            }
        }
    }

    fn device_uuid(&self) -> Result<String, AmdError> {
        let mut uuid_buffer = vec![0 as c_char; AMDSMI_GPU_UUID_SIZE as usize];
        let mut uuid_length = AMDSMI_GPU_UUID_SIZE;

        // SAFETY: According to AMD-SMI documentation, the function will not write beyond `uuid_length`.
        // `uuid_length` must be initialized to the buffer size, and the function will update it with the actual length.
        let result = unsafe {
            self.amdsmi.amdsmi.amdsmi_get_gpu_device_uuid(
                self.inner,
                &mut uuid_length,
                uuid_buffer.as_mut_ptr(),
            )
        };

        self.amdsmi.check_status(result)?;

        // SAFETY: Create a `CStr` from the FFI buffer.
        // If the buffer already ends with a null terminator, we use it directly.
        // Otherwise, we copy into a local stack buffer and append a null terminator that ensures `from_ptr` receives a null-terminated C string.
        let c_str = if uuid_buffer[(uuid_length - 1) as usize] == 0 {
            unsafe { CStr::from_ptr(uuid_buffer.as_ptr()) }
        } else {
            let mut cstr_buffer = [0 as c_char; AMDSMI_GPU_UUID_SIZE as usize + 1];
            cstr_buffer[..uuid_length as usize]
                .copy_from_slice(&uuid_buffer[..uuid_length as usize]);
            cstr_buffer[uuid_length as usize] = 0;
            unsafe { CStr::from_ptr(cstr_buffer.as_ptr()) }
        };

        c_str.to_str().map(|s| s.to_owned()).map_err(|_| AmdError {
            status: result,
            message: None,
        })
    }
}
