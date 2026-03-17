use std::{mem::zeroed, ptr::null_mut, sync::Arc};

#[cfg(feature = "mock")]
use mockall::automock;

mod bindings;
pub mod error;
pub mod handles;
pub mod metrics;

use crate::handles::AmdSocketHandle;
use error::{AmdError, AmdInitError};

pub(crate) const LIB_PATH: &str = "libamd_smi.so";

/// Initialization flags for the library.
/// See [`AmdSmi::init`].
pub type AmdInitFlags = crate::bindings::amdsmi_init_flags_t;

/// Main wrapper around the AMD SMI library.
///
/// # Shutdown
/// The library is automatically shut down when `AmdSmi` is dropped.
/// The `Drop` implementation of `AmdSmi` ignores shutdown errors.
/// To handle the error, call [`AmdInterface::stop`].
pub struct AmdSmi {
    amdsmi: bindings::libamd_smi,
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
    /// Checks the value of [`amdsmi_status_t`] to return `Ok` or `Err`.
    fn check_status(&self, status: bindings::amdsmi_status_t) -> Result<(), AmdError> {
        match status {
            bindings::amdsmi_status_t::AMDSMI_STATUS_SUCCESS => Ok(()),
            status => Err(AmdError {
                status,
                message: error::message_for_status(&self.amdsmi, status),
            }),
        }
    }

    /// Initializes the AMD smi library.
    ///
    /// # Example
    /// ```no_run
    /// use amd_smi_wrapper::{AmdSmi, AmdInitFlags};
    ///
    /// let amdsmi = AmdSmi::init(AmdInitFlags::AMDSMI_INIT_AMD_GPUS).expect("init failed");
    /// ```
    pub fn init(flags: AmdInitFlags) -> Result<Arc<Self>, AmdInitError> {
        // SAFETY: The library must exist at the specified path, otherwise `libamd_smi::new` returns an error.
        // This operation involves raw FFI interaction and assumes the dynamic loader succeeds.
        let amdsmi = unsafe { bindings::libamd_smi::new(LIB_PATH)? };
        let instance = Arc::new(AmdSmi { amdsmi });

        // SAFETY: The function expects a valid library instance and valid flags.
        // According to the AMD-SMI documentation, the function fully initializes internal structures for GPU discovery.
        // The return code `amdsmi_status_t` is checked to ensure initialization succeeded before using the library.
        let status = unsafe { instance.amdsmi.amdsmi_init(flags.0.into()) };
        instance.check_status(status)?;

        Ok(instance)
    }
}

/// Provides AMD SMI functions.
///
/// The actual implementation is [`AmdSmi`].
/// In tests, you can use the mock implementation `MockAmdInterface` (requires the `mock` feature).
#[cfg_attr(feature = "mock", automock(type SocketHandle=handles::MockSocketHandle;))]
pub trait AmdInterface {
    /// Type of socket handle managed by this interface.
    type SocketHandle: handles::SocketHandle;

    /// Stops the AMD SMI library.
    fn stop(self) -> Result<(), AmdError>;

    /// Lists the available sockets.
    ///
    /// Only the sockets that match the initialization flags are returned.
    /// For instance, if the library has been initialized with [`AMDSMI_INIT_AMD_GPUS`](AmdInitFlags::AMDSMI_INIT_AMD_GPUS),
    /// only sockets with GPUs are returned.
    fn socket_handles(&self) -> Result<Vec<Self::SocketHandle>, AmdError>;
}

impl AmdInterface for Arc<AmdSmi> {
    type SocketHandle = AmdSocketHandle;

    fn stop(self) -> Result<(), AmdError> {
        // Shut down the AMD-SMI library and release all internal resources.
        // SAFETY: The function expects a valid, initialized library instance.
        // The Arc ensures that shutdown is only called once when the last reference is dropped.
        let result = unsafe { self.amdsmi.amdsmi_shut_down() };
        self.check_status(result)
    }

    fn socket_handles(&self) -> Result<Vec<Self::SocketHandle>, AmdError> {
        let mut socket_count = 0;

        // Query the number of available GPU socket handles.
        // SAFETY: According to the AMD-SMI documentation, passing `null_mut()` is safe which sets `socket_count` to the number of sockets in the system.
        let result = unsafe {
            self.amdsmi
                .amdsmi_get_socket_handles(&mut socket_count, null_mut())
        };
        self.check_status(result)?;

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
        self.check_status(result)?;

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
