# Rust wrapper for AMD SMI library

🚧 **WARNING**: This wrapper is work in progress, use at your own risk.

- **Header**: `ROCm 7.2.0`
- **Crate**: `0.2.1`

Currently includes and implements the following elements:

|Resource|Description|
|--------|-----------|
|`amdsmi_status_t`|Return errors and status codes for a given AMD-SMI command|
|`amdsmi_clk_type_t`|Existing clock devices on a AMD hardware|
|`amdsmi_memory_type_t`|Existing memory devices on a AMD hardware|
|`amdsmi_temperature_type_t`|Existing thermal sensors on an AMD hardware|
|`amdsmi_temperature_metric_t`|Thermal monitoring type for a given sensor|
|`amdsmi_voltage_type_t`|Existing voltage probes on an AMD hardware|
|`amdsmi_voltage_metric_t`|Voltage monitoring type for a given probe|

|Features|Description|
|--------|-----------|
|`amdsmi_get_clock_info`|Clock frequency of a give GPU|
|`amdsmi_get_energy_count`|Energy consumption of a given GPU|
|`amdsmi_get_gpu_activity`|Activity of engine unit of a given GPU|
|`amdsmi_get_gpu_device_uuid`|Hardware ID of a given GPU|
|`amdsmi_get_gpu_fan_speed`|Fan speed of a given GPU|
|`amdsmi_get_gpu_memory_usage`|Memory consumption of a given GPU|
|`amdsmi_get_gpu_pci_throughput`|PCI bus traffic by a given GPU|
|`amdsmi_get_gpu_process_list`|GPU metrics usage by running processes|
|`amdsmi_get_gpu_volt_metric`|Voltage of a given GPU|
|`amdsmi_get_power_info`|Power consumption of a given GPU|
|`amdsmi_get_temp_metric`|Temperature emitted by given GPU areas|
|`amdsmi_is_gpu_power_management_enabled`|Status required to retrieve the power consumption for a given GPU|

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
