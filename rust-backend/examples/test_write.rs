use hidapi::HidApi;
use std::thread;
use std::time::Duration;

const VID: u16 = 0x048d;
const PID: u16 = 0xc693;

fn main() {
    println!("Initializing HID API...");
    let api = HidApi::new().expect("Failed to init HID API");
    
    let device_info = api.device_list()
        .find(|d| d.vendor_id() == VID && d.product_id() == PID && d.interface_number() == 1)
        .expect("Device not found on interface 1");
    
    println!("Opening device at path: {:?}", device_info.path());
    let device = api.open_path(device_info.path()).expect("Failed to open device");
    
    println!("Testing Report ID 5 (Set Range) with exact size (10 bytes)...");
    // Report 5 format: [0x05, 0x01, start, 0x00, end, 0x00, R, G, B, 0x01]
    // Let's set zones 0 to 23 to green (0, 255, 0)
    let buf_exact = vec![0x05, 0x01, 0x00, 0x00, 0x17, 0x00, 0x00, 0xff, 0x00, 0x01];
    println!("Sending packet: {:?}", buf_exact);
    match device.send_feature_report(&buf_exact) {
        Ok(()) => println!("Success!"),
        Err(e) => eprintln!("Error sending feature report: {}", e),
    }

    thread::sleep(Duration::from_millis(500));

    println!("Testing Report ID 5 (Set Range) with exact size (10 bytes) - Setting to blue...");
    let buf_exact_blue = vec![0x05, 0x01, 0x00, 0x00, 0x17, 0x00, 0x00, 0x00, 0xff, 0x01];
    println!("Sending packet: {:?}", buf_exact_blue);
    match device.send_feature_report(&buf_exact_blue) {
        Ok(()) => println!("Success!"),
        Err(e) => eprintln!("Error sending feature report: {}", e),
    }

    println!("Done!");
}
