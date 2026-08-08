use hidapi::HidApi;
use std::thread;
use std::time::Duration;

const VID: u16 = 0x048d;
const PID_NORMAL: u16 = 0xc693;
const PID_UPGRADE: u16 = 0x89db;

fn main() {
    println!("Initializing HID API...");
    let api = match HidApi::new() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("❌ Failed to initialize HID API: {}", e);
            std::process::exit(1);
        }
    };

    // Check if the device is in Upgrade Mode first
    let has_upgrade = api.device_list().any(|d| d.vendor_id() == VID && d.product_id() == PID_UPGRADE);
    if has_upgrade {
        eprintln!("⚠️ WARNING: Lenovo LOQ RGB Controller is in UPGRADE MODE (048d:89db)!");
        eprintln!("   In this mode, host control is disabled and the firmware has crashed/dropped into bootloader.");
        eprintln!("   To recover: Shut down the laptop, unplug the power adapter for 10 seconds, then power back on.");
        std::process::exit(2);
    }

    // Find the normal RGB controller device on interface 1
    let device_info = api.device_list()
        .find(|d| d.vendor_id() == VID && d.product_id() == PID_NORMAL && d.interface_number() == 1);

    let info = match device_info {
        Some(i) => i,
        None => {
            eprintln!("❌ Device not found on interface 1 (VID: 048d, PID: c693).");
            eprintln!("   Make sure your keyboard is plugged in and you have appropriate permissions (udev rules).");
            std::process::exit(3);
        }
    };

    println!("Found RGB Controller at path: {:?}", info.path());
    
    let device = match api.open_path(info.path()) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("❌ Failed to open device path: {}", e);
            eprintln!("   This is likely a permission issue or the device is already in use by another driver.");
            eprintln!("   Try stopping the rgb-server first or check your udev rules.");
            std::process::exit(4);
        }
    };

    println!("Sending command to disable autonomous mode (taking host control)...");
    let auto_off = vec![0x06, 0x00];
    match device.send_feature_report(&auto_off) {
        Ok(_) => {
            thread::sleep(Duration::from_millis(15));
            println!("✅ Successfully disabled autonomous mode! Keyboard is now host-controlled.");
        }
        Err(e) => {
            eprintln!("❌ Failed to send autonomous mode disable command: {}", e);
            std::process::exit(5);
        }
    }
}
