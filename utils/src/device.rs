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

//function to take a default device name and set the device we are using to that,
//or it finds the default innput device from host and uses that
pub fn get_or_default_input(device_name: Option<String>) -> anyhow::Result<Device> {
    //get the host interface
    let host = get_host();
    //track the id of the host
    tracing::debug!("Host: {:?}", host.id());
    //set the target name of our device to the provided parameter or default input device
    let target = device_name
        .clone()
        .unwrap_or_else(|| host.default_input_device().unwrap().name().unwrap());

    //create our device object but set it to None
    let mut device: Option<Device> = None;
    //get all the input devices from the host
    let input_devices = host.input_devices().unwrap();
    for in_device in input_devices {
        //if the device exists on our host interfaces list of devices then create the object
        if in_device.name().is_ok_and(|name| name == target) {
            device = Some(in_device);
        }
    }
    //if it doesnt exsits output this error
    if device.is_none() {
        return Err(anyhow::anyhow!("No target device found"));
    }
    //unwrap the device and return it as a result
    let device = device.unwrap();
    Ok(device)
}

//does the same as the input device above
pub fn get_or_default_output(device_name: Option<String>) -> anyhow::Result<Device> {
    let host = get_host();
    let target = device_name
        .clone()
        .unwrap_or_else(|| host.default_output_device().unwrap().name().unwrap());

    let mut device: Option<Device> = None;
    let output_devices = host.output_devices().unwrap();
    for out_device in output_devices {
        if out_device.name().is_ok_and(|name| name == target) {
            device = Some(out_device);
        }
    }
    if device.is_none() {
        return Err(anyhow::anyhow!("No target device found"));
    }
    let device = device.unwrap();
    Ok(device)
}

pub fn get_available_inputs() -> String {
    for host in cpal::available_hosts() {
        tracing::debug!("Available host: {:?}", host);
    }

    let host = get_host();

    let mut device_names: Vec<String> = Vec::new();
    //get the default device and list and list its name with error handling
    let default_device = host
        .default_input_device()
        .expect("No default input device")
        .name()
        .expect("Default input device has no name...");
    //error handling for checking for input devices
    let input_devices = host.input_devices().expect("No input devices found");
    for in_device in input_devices {
        let d_name = in_device.name().expect("Device has no name...");
        //checl the devices configuratinso
        let d_cfg = in_device
            .default_input_config()
            .expect("Device has no default input config...");
        let d_sampling_rate = d_cfg.sample_rate().0;
        let d_ch = d_cfg.channels();
        
        //output the string of configs
        let mut d = format!(" * {}({}ch, {}hz)", d_name, d_ch, d_sampling_rate);
        if d_name == default_device {
            d.push_str(" [default]");
        }
        device_names.push(d);
    }
    //return a string of all audio configs
    device_names.join("\n")
}

//does the same as the input function above
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