use cpal::Device;
use cpal::traits::{DeviceTrait, HostTrait};

fn get_host() -> cpal::Host {
    // if cfg!(target_os = "windows") {
    //     cpal::host_from_id(cpal::HostId::Asio).expect("failed to initialise ASIO host")
    // } else {
    //     cpal::default_host()
    // }
    cpal::default_host()
}

/// Gets a specific input device by name, or falls back to the host's default input device.
pub fn get_or_default_input(device_name: Option<String>) -> anyhow::Result<Device> {
    // Get the audio host interface.
    let host = get_host();
    // Track the ID of the host for debugging.
    tracing::debug!("Host: {:?}", host.id());
    // Set the target device name to the provided parameter or the default input device.
    let target = device_name
        .clone()
        .unwrap_or_else(|| host.default_input_device().unwrap().name().unwrap());

    // Search for the target device among available input devices.
    let mut device: Option<Device> = None;
    let input_devices = host.input_devices()?;
    for in_device in input_devices {
        // If the device name matches our target, select it.
        if in_device.name().is_ok_and(|name| name == target) {
            device = Some(in_device);
            break;
        }
    }
    // If the device wasn't found, return an error.
    if device.is_none() {
        return Err(anyhow::anyhow!("Target input device not found: {}", target));
    }
    // Unwrap the device and return it.
    let device = device.unwrap();
    Ok(device)
}

/// Gets a specific output device by name, or falls back to the host's default output device.
pub fn get_or_default_output(device_name: Option<String>) -> anyhow::Result<Device> {
    let host = get_host();
    let target = device_name
        .clone()
        .unwrap_or_else(|| host.default_output_device().unwrap().name().unwrap());

    let mut device: Option<Device> = None;
    let output_devices = host.output_devices()?;
    for out_device in output_devices {
        if out_device.name().is_ok_and(|name| name == target) {
            device = Some(out_device);
            break;
        }
    }
    if device.is_none() {
        return Err(anyhow::anyhow!(
            "Target output device not found: {}",
            target
        ));
    }
    let device = device.unwrap();
    Ok(device)
}

/// Returns a formatted string listing all available audio input devices.
pub fn get_available_inputs() -> String {
    for host in cpal::available_hosts() {
        tracing::debug!("Available host: {:?}", host);
    }

    let host = get_host();

    let mut device_names: Vec<String> = Vec::new();
    // Get the default device name for comparison.
    let default_device = host
        .default_input_device()
        .expect("No default input device")
        .name()
        .expect("Default input device has no name...");
    // Iterate through all found input devices.
    let input_devices = host.input_devices().expect("No input devices found");
    for in_device in input_devices {
        let d_name = in_device.name().expect("Device has no name...");
        // Check the device's default configuration.
        let d_cfg = in_device
            .default_input_config()
            .expect("Device has no default input config...");
        let d_sampling_rate = d_cfg.sample_rate().0;
        let d_ch = d_cfg.channels();

        // Format the output string.
        let mut d = format!(" * {}({}ch, {}hz)", d_name, d_ch, d_sampling_rate);
        if d_name == default_device {
            d.push_str(" [default]");
        }
        device_names.push(d);
    }
    // Return a single string with each device on a new line.
    device_names.join("\n")
}

/// Returns a formatted string listing all available audio output devices.
pub fn get_available_outputs() -> String {
    for host in cpal::available_hosts() {
        tracing::debug!("Available host: {:?}", host);
    }

    let host = get_host();
    let mut device_names: Vec<String> = Vec::new();
    let default_device = host
        .default_output_device()
        .expect("No default output device")
        .name()
        .expect("Default output device has no name...");
    let output_devices = host.output_devices().expect("No output devices found");
    for out_device in output_devices {
        let d_name = out_device.name().expect("Device has no name...");
        let d_cfg = out_device
            .default_output_config()
            .expect("Device has no default output config...");
        let d_sampling_rate = d_cfg.sample_rate().0;
        let d_ch = d_cfg.channels();

        let mut d = format!(" * {}({}ch, {}hz)", d_name, d_ch, d_sampling_rate);
        if d_name == default_device {
            d.push_str(" [default]");
        }
        device_names.push(d);
    }
    device_names.join("\n")
}