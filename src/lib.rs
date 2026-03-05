#[cfg(feature = "mock")]
use mockall::automock;
use std::{
    ffi::CStr,
    mem::{MaybeUninit, zeroed},
    os::raw::c_char,
    ptr::null_mut,
    sync::Arc,
};
use thiserror::Error;

pub mod utils;

use crate::bindings::*;
use crate::utils::*;

#[allow(warnings)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

const LIB_PATH: &str = "libamd_smi.so";

/// Error while using the AMD SMI library.
///
/// This wraps an [`amdsmi_status_t`] provided by the underlying C functions.
#[derive(Debug, Error)]
#[error("amd-smi library error: {0:?}")]
pub struct AmdError(pub AmdStatus);

#[derive(Debug, Error)]
pub enum AmdInitError {
    #[error("amd-smi init error")]
    Init(#[from] AmdError),
    #[error("Failed to load {LIB_PATH}")]
    Load(#[from] libloading::Error),
}

pub struct AmdSmi {
    amdsmi: libamd_smi,
}

pub struct AmdSocketHandle {
    amdsmi: Arc<AmdSmi>,
    inner: amdsmi_socket_handle,
}

pub struct AmdProcessorHandle {
    amdsmi: Arc<AmdSmi>,
    inner: amdsmi_processor_handle,
}

/// Checking the value of [`amdsmi_status_t`] to return an error or success.
fn check_status(status: amdsmi_status_t) -> Result<(), AmdError> {
    if status == AmdStatus::AMDSMI_STATUS_SUCCESS {
        Ok(())
    } else {
        Err(AmdError(status))
    }
}

impl Drop for AmdSmi {
    fn drop(&mut self) {
        // Shut down the AMD-SMI library and release all internal resources.
        // SAFETY: The function expects a valid, initialized library instance.
        // The shutdown is called only once when the last reference is dropped.
        unsafe { self.amdsmi.amdsmi_shut_down() };
    }
}

impl AmdSmi {
    /// Initialize and start amd-smi library with [`InitFlags::AMD_GPUS`].
    pub fn init(flags: InitFlags) -> Result<Arc<Self>, AmdInitError> {
        // SAFETY: The library must exist at the specified path, otherwise `libamd_smi::new` returns an error.
        // This operation involves raw FFI interaction and assumes the dynamic loader succeeds.
        let amdsmi = unsafe { libamd_smi::new(LIB_PATH)? };

        // SAFETY: The function expects a valid library instance and valid flags.
        // According to the AMD-SMI documentation, the function fully initializes internal structures for GPU discovery.
        // The return code `amdsmi_status_t` is checked to ensure initialization succeeded before using the library.
        let result = unsafe { amdsmi.amdsmi_init(flags.bits().into()) };
        check_status(result).map_err(AmdInitError::Init)?;

        Ok(Arc::new(AmdSmi { amdsmi }))
    }
}

#[cfg_attr(feature = "mock", automock(type SocketHandle=MockSocketHandle;))]
pub trait AmdInterface {
    type SocketHandle: SocketHandle;
    /// Quit amd-smi library and clean properly its resources.
    fn stop(self) -> Result<(), AmdError>;

    /// Retrieves a set of [`SocketHandle`] structure containing socket handles associated to a GPU device.
    fn socket_handles(&self) -> Result<Vec<Self::SocketHandle>, AmdError>;
}

impl AmdInterface for Arc<AmdSmi> {
    type SocketHandle = AmdSocketHandle;

    /// Quit amd-smi library and clean properly its resources.
    fn stop(self) -> Result<(), AmdError> {
        // Shut down the AMD-SMI library and release all internal resources.
        // SAFETY: The function expects a valid, initialized library instance.
        // The Arc ensures that shutdown is only called once when the last reference is dropped.
        let result = unsafe { self.amdsmi.amdsmi_shut_down() };
        check_status(result)
    }

    /// Retrieves a set of [`SocketHandle`] structure containing socket handles associated to a GPU device.
    fn socket_handles(&self) -> Result<Vec<Self::SocketHandle>, AmdError> {
        let mut socket_count = 0;

        // Query the number of available GPU socket handles.
        // SAFETY: According to the AMD-SMI documentation, passing `null_mut()` is safe which sets `socket_count` to the number of sockets in the system.
        let result = unsafe {
            self.amdsmi
                .amdsmi_get_socket_handles(&mut socket_count, null_mut())
        };
        check_status(result)?;

        // Allocate an uninitialized vector of socket handles.
        // SAFETY: Each element is zeroed and considered valid for the FFI call and AMD-SMI library will fill each handle in the second call.
        let mut socket_handles = vec![unsafe { zeroed() }; socket_count as usize];

        // Fill the buffer with socket handles.
        // SAFETY: `socket_handles.as_mut_ptr()` points to memory of sufficient size.
        // According the AMD-SMI library documentation, the function writes at most `socket_count` handles, so no out-of-bounds write occurs.
        let result = unsafe {
            self.amdsmi
                .amdsmi_get_socket_handles(&mut socket_count, socket_handles.as_mut_ptr())
        };
        check_status(result)?;

        socket_handles.truncate(socket_count as usize);

        Ok(socket_handles
            .into_iter()
            .map(|s| AmdSocketHandle {
                amdsmi: Arc::clone(self),
                inner: s,
            })
            .collect())
    }
}

#[cfg_attr(feature = "mock", automock(type ProcessorHandle=MockProcessorHandle;))]
pub trait SocketHandle {
    type ProcessorHandle: ProcessorHandle;

    /// Retrieves a set of [`ProcessorHandle`] structure containing processor handles associated to a GPU device.
    fn processor_handles(&self) -> Result<Vec<Self::ProcessorHandle>, AmdError>;
}

impl SocketHandle for AmdSocketHandle {
    type ProcessorHandle = AmdProcessorHandle;

    /// Retrieves a set of [`ProcessorHandle`] structure containing processor handles associated to a GPU device.
    fn processor_handles(&self) -> Result<Vec<Self::ProcessorHandle>, AmdError> {
        let mut processor_count = 0;

        // Query the number of processor handles for the given socket.
        // SAFETY: According the AMD-SMI library documentation, passing `null_mut()` is safe which sets `processor_count` to the number of processors available for this socket.
        let result = unsafe {
            self.amdsmi.amdsmi.amdsmi_get_processor_handles(
                self.inner,
                &mut processor_count,
                null_mut(),
            )
        };
        check_status(result)?;

        // Allocate an uninitialized vector of socket handles.
        // SAFETY: Each element is zeroed and considered valid for the FFI call and AMD-SMI library will fill each handle in the second call.
        let mut processor_handles = vec![unsafe { zeroed() }; processor_count as usize];

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
        check_status(result)?;
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

#[cfg_attr(feature = "mock", automock)]
pub trait ProcessorHandle {
    /// Retrieves the UUID of the GPU device.
    fn device_uuid(&self) -> Result<String, AmdError>;

    /// Retrieves a [`AmdEngineUsage`] structure containing all data about GPU device activities.
    fn device_activity(&self) -> Result<AmdEngineUsage, AmdError>;

    /// Retrieves the energy consumption of the GPU device.
    fn device_energy_consumption(&self) -> Result<AmdEnergyConsumption, AmdError>;

    /// Retrieves for a given [`AmdMemoryType`] the memory consumption of the GPU device.
    fn device_memory_usage(&self, mem_type: AmdMemoryType) -> Result<u64, AmdError>;

    /// Retrieves a [`AmdPowerConsumption`] structure containing all data about GPU device power consumption.
    fn device_power_consumption(&self) -> Result<AmdPowerConsumption, AmdError>;

    /// Retrieves the power management status accessability of the GPU device.
    fn device_power_managment(&self) -> Result<bool, AmdError>;

    /// Retrieves the temperature of a given area of the GPU device.
    ///
    /// # Arguments
    ///
    /// - `sensor_type`: Temperature retrieved by a [`AmdTemperatureType`] sensor on AMD GPU hardware.
    /// - `metric`: Temperature type [`AmdTemperatureMetric`] analyzed (current, average...).
    fn device_temperature(
        &self,
        sensor_type: AmdTemperatureType,
        metric: AmdTemperatureMetric,
    ) -> Result<i64, AmdError>;

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

    /// Retrieves a set of [`AmdProcess`] structure containing data about running processes on the GPU device.
    fn device_process_list(&self) -> Result<Vec<AmdProcess>, AmdError>;
}

impl ProcessorHandle for AmdProcessorHandle {
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

        check_status(result)?;

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

        c_str
            .to_str()
            .map(|s| s.to_owned())
            .map_err(|_| AmdError(result))
    }

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

        check_status(result)?;

        // SAFETY: `assume_init()` is safe because the FFI call succeeded and fully initialized `info`.
        Ok(unsafe { info.assume_init().into() })
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

        check_status(result)?;
        Ok(consumption)
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

        check_status(result)?;
        Ok(used)
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

        check_status(result)?;

        // SAFETY: `assume_init()` is safe because the FFI call returned SUCCESS, meaning `info` is fully initialized.
        Ok(unsafe { info.assume_init().into() })
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

        check_status(result)?;
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

        check_status(result)?;
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

        check_status(result)?;
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
                null_mut(),
            )
        };

        match result {
            AmdStatus::AMDSMI_STATUS_SUCCESS => {}
            AmdStatus::AMDSMI_STATUS_OUT_OF_RESOURCES => {}
            err => return Err(AmdError(err)),
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
                AmdStatus::AMDSMI_STATUS_SUCCESS => unsafe {
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
                AmdStatus::AMDSMI_STATUS_OUT_OF_RESOURCES => {
                    max_processes = count;
                    continue;
                }
                err => return Err(AmdError(err)),
            }
        }
    }
}
