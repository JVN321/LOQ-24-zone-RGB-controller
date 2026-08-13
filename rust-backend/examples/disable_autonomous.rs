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

    println!("Executing LampArray discovery handshake protocol...");

    // Step 1: Query LampArrayAttributesReport (Report 0x01)
    println!(" 1. Querying LampArrayAttributesReport (Report 0x01)...");
    let mut buf1 = [0u8; 64];
    buf1[0] = 0x01;
    match device.get_feature_report(&mut buf1) {
        Ok(n) => println!("    ✓ Received LampArray attributes ({} bytes)", n),
        Err(e) => eprintln!("    ⚠️ Warning querying Report 0x01: {}", e),
    }
    thread::sleep(Duration::from_millis(15));

    // Step 2: Query LampAttributesRequestReport (Report 0x02 -> 0x03)
    println!(" 2. Requesting Lamp 0 attributes (Report 0x02)...");
    let req2 = vec![0x02, 0x00, 0x00];
    if let Err(e) = device.send_feature_report(&req2) {
        eprintln!("    ⚠️ Warning sending Report 0x02: {}", e);
    }
    thread::sleep(Duration::from_millis(15));

    let mut buf3 = [0u8; 64];
    buf3[0] = 0x03;
    let _ = device.get_feature_report(&mut buf3);
    thread::sleep(Duration::from_millis(15));

    // Step 3: Send LampArrayControlReport (Report 0x06: AutonomousMode = 0)
    println!(" 3. Sending LampArrayControlReport (Report 0x06: AutonomousMode = 0)...");
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
