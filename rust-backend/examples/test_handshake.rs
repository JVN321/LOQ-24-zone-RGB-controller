// test_handshake.rs
// Test tool to experiment with LampArray initialization sequences on ITE 8258 (048d:c693)

use hidapi::HidApi;
use std::thread;
use std::time::Duration;

const VID: u16 = 0x048d;
const PID: u16 = 0xc693;

fn main() {
    println!("=== LOQ RGB Controller — LampArray Handshake Test ===");

    let api = match HidApi::new() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to init HID API: {}", e);
            return;
        }
    };

    let dev_info = api
        .device_list()
        .find(|d| d.vendor_id() == VID && d.product_id() == PID && d.interface_number() == 1);

    let info = match dev_info {
        Some(i) => i,
        None => {
            eprintln!("Device 048d:c693 interface 1 not found!");
            return;
        }
    };

    println!("Opening device at path: {:?}", info.path());
    let device = match api.open_path(info.path()) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to open device: {}", e);
            return;
        }
    };

    // 1. Try reading Report 1 (LampArrayAttributesReport)
    println!("\n--- Step 1: Query Report 1 (LampArrayAttributesReport) ---");
    let mut buf1 = [0u8; 64];
    buf1[0] = 0x01;
    match device.get_feature_report(&mut buf1) {
        Ok(n) => println!("Got Report 1 ({} bytes): {:?}", n, &buf1[..n]),
        Err(e) => eprintln!("Get Feature Report 1 failed: {}", e),
    }

    thread::sleep(Duration::from_millis(20));

    // 2. Try requesting Report 2 & reading Report 3 for Lamp 0
    println!("\n--- Step 2: Request Lamp 0 Attributes (Report 2 -> Report 3) ---");
    let req2 = vec![0x02, 0x00, 0x00]; // Lamp ID 0 (16-bit)
    match device.send_feature_report(&req2) {
        Ok(_) => println!("Sent Report 2 (LampAttributesRequest) for Lamp 0"),
        Err(e) => eprintln!("Send Report 2 failed: {}", e),
    }

    thread::sleep(Duration::from_millis(20));

    let mut buf3 = [0u8; 64];
    buf3[0] = 0x03;
    match device.get_feature_report(&mut buf3) {
        Ok(n) => println!("Got Report 3 (LampAttributesResponse, {} bytes): {:?}", n, &buf3[..n]),
        Err(e) => eprintln!("Get Feature Report 3 failed: {}", e),
    }

    thread::sleep(Duration::from_millis(20));

    // 3. Send Report 6 (LampArrayControlReport: AutonomousMode = 0)
    println!("\n--- Step 3: Send Report 6 (Disable Autonomous Mode) ---");
    let ctrl6 = vec![0x06, 0x00];
    match device.send_feature_report(&ctrl6) {
        Ok(_) => println!("Sent Report 6 [0x06, 0x00] (AutonomousMode = 0)"),
        Err(e) => eprintln!("Send Report 6 failed: {}", e),
    }

    thread::sleep(Duration::from_millis(20));

    // 4. Try reading Report 6 back
    let mut buf6 = [0u8; 64];
    buf6[0] = 0x06;
    match device.get_feature_report(&mut buf6) {
        Ok(n) => println!("Read Report 6 back ({} bytes): {:?}", n, &buf6[..n]),
        Err(e) => eprintln!("Get Feature Report 6 failed: {}", e),
    }

    thread::sleep(Duration::from_millis(20));

    // 5. Send Report 5 (Set solid Red on all zones 0-23)
    println!("\n--- Step 5: Send Report 5 (LampRangeUpdateReport - Solid Red) ---");
    let range5 = vec![
        0x05, // Report ID 5
        0x01, // Zone range subcommand
        0x00, // Start zone 0
        0x00, // Reserved
        0x17, // End zone 23 (0x17)
        0x00, // Reserved
        0xFF, // R = 255
        0x00, // G = 0
        0x00, // B = 0
        0x01, // Commit
    ];
    match device.send_feature_report(&range5) {
        Ok(_) => println!("Sent Report 5 (Set All Red)"),
        Err(e) => eprintln!("Send Report 5 failed: {}", e),
    }

    println!("\nTest sequence complete!");
}
