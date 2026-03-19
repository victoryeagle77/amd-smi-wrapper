use amd_smi_wrapper::{
    AmdInitFlags, AmdInterface, AmdSmi,
    handles::{ProcessorHandle, SocketHandle},
};

fn skip_gpu_tests() -> bool {
    if std::env::var_os("NO_GPU").is_some() {
        println!("test skipped because NO_GPU is set");
        true
    } else {
        false
    }
}

#[test]
fn list_devices() {
    if skip_gpu_tests() {
        return;
    }

    let amdsmi = AmdSmi::init(AmdInitFlags::AMDSMI_INIT_AMD_GPUS).unwrap();
    for socket in amdsmi.socket_handles().unwrap() {
        for proc in socket.processor_handles().unwrap() {
            let uuid = proc.device_uuid().unwrap();
            println!("found gpu: {uuid}");
        }
    }
    // automatic drop of amdsmi
}

#[test]
fn explicit_stop() {
    if skip_gpu_tests() {
        return;
    }

    let amdsmi = AmdSmi::init(AmdInitFlags::AMDSMI_INIT_AMD_GPUS).unwrap();
    amdsmi.stop().expect("error in stop");
}
