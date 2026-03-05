# Rust wrapper for AMD SMI library

🚧 Warning: this is a work in progress. Use at your own risk.

Currently includes and implements the following features:

|Ressource|Description|
|---------|-----------|
|`amdsmi_status_t`|Return errors and status codes for a given AMD-SMI command|
|`amdsmi_clk_type_t`|Existing clock devices on a AMD hardware|
|`amdsmi_memory_type_t`|Existing memory devices on a AMD hardware|
|`amdsmi_temperature_type_t`|Existing thermal sensors on an AMD hardware|
|`amdsmi_temperature_metric_t`|Thermal monitoring type for a given sensor|
|`amdsmi_voltage_type_t`|Existing voltage probes on an AMD hardware|
|`amdsmi_voltage_metric_t`|Voltage monitoring type for a given probe|

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
