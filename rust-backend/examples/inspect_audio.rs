use cpal::traits::{DeviceTrait, HostTrait};

fn main() {
    let host = cpal::default_host();
    println!("Default input device: {:?}", host.default_input_device().and_then(|d| d.name().ok()));
    println!("Default output device: {:?}", host.default_output_device().and_then(|d| d.name().ok()));
    
    println!("\nAll input devices:");
    if let Ok(devices) = host.input_devices() {
        for (i, dev) in devices.enumerate() {
            println!("  Input {}: {:?}", i, dev.name().unwrap_or_else(|_| "Unknown".to_string()));
        }
    } else {
        println!("  None found");
    }

    println!("\nAll output devices:");
    if let Ok(devices) = host.output_devices() {
        for (i, dev) in devices.enumerate() {
            println!("  Output {}: {:?}", i, dev.name().unwrap_or_else(|_| "Unknown".to_string()));
        }
    } else {
        println!("  None found");
    }
}
