use bindgen::{Builder, callbacks::ParseCallbacks};
use std::{env::var, path::PathBuf};

const LIB: &str = "libamd_smi";
const HEADER: &str = "include/amdsmi.h";

#[derive(Debug)]
struct DocFix;

impl ParseCallbacks for DocFix {
    fn process_comment(&self, comment: &str) -> Option<String> {
        // Transform C/C++ documentation to avoid Rust doc-test errors
        Some(format!("```text\n{comment}\n```"))
    }
}

fn main() {
    let out_path = PathBuf::from(var("OUT_DIR").unwrap());

    Builder::default()
        .header(HEADER)
        .parse_callbacks(Box::new(DocFix))
        .dynamic_library_name(LIB)
        .newtype_enum("amdsmi_status_t")
        .newtype_enum("amdsmi_memory_type_t")
        .newtype_enum("amdsmi_temperature_type_t")
        .newtype_enum("amdsmi_temperature_metric_t")
        .newtype_enum("amdsmi_voltage_type_t")
        .newtype_enum("amdsmi_voltage_metric_t")
        .newtype_enum("amdsmi_clk_type_t")
        .bitfield_enum("amdsmi_init_flags_t")
        .generate()
        .expect("bindgen failed")
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Failed to write bindings");
}
