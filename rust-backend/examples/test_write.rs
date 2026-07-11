use hidapi::HidApi;
use std::thread;
use std::time::Duration;

const VID: u16 = 0x048d;
const PID: u16 = 0xc693;
const PACKET_SIZE: usize = 65;

fn main() {
    println!("Initializing HID API...");
    let api = HidApi::new().expect("Failed to init HID API");
    
    let device_info = api.device_list()
        .find(|d| d.vendor_id() == VID && d.product_id() == PID && d.interface_number() == 1)
        .expect("Device not found on interface 1");
    
    println!("Opening device at path: {:?}", device_info.path());
    let device = api.open_path(device_info.path()).expect("Failed to open device");

    println!("Disabling autonomous mode (taking host control)...");
    let auto_off = vec![0x06, 0x00];
    device.send_feature_report(&auto_off).expect("Failed to disable autonomous mode");
    thread::sleep(Duration::from_millis(10));
    
    println!("Setting zones 0 to 23 to GREEN (padded to 65 bytes)...");
    // Report 5 format: [0x05, 0x01, start, 0x00, end, 0x00, R, G, B, 0x01]
    let mut buf_green = vec![0x05, 0x01, 0x00, 0x00, 0x17, 0x00, 0x00, 0xff, 0x00, 0x01];
    buf_green.resize(PACKET_SIZE, 0);
    device.send_feature_report(&buf_green).expect("Failed to send green color");
    println!("Green color sent!");
    
    thread::sleep(Duration::from_secs(3));

    println!("Setting zones 0 to 23 to BLUE (padded to 65 bytes)...");
    let mut buf_blue = vec![0x05, 0x01, 0x00, 0x00, 0x17, 0x00, 0x00, 0x00, 0xff, 0x01];
    buf_blue.resize(PACKET_SIZE, 0);
    device.send_feature_report(&buf_blue).expect("Failed to send blue color");
    println!("Blue color sent!");

    thread::sleep(Duration::from_secs(3));

    println!("Enabling autonomous mode (giving back control)...");
    let auto_on = vec![0x06, 0x01];
    device.send_feature_report(&auto_on).expect("Failed to enable autonomous mode");
    thread::sleep(Duration::from_millis(10));

    println!("Done!");
}
