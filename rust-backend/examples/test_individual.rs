use hidapi::HidApi;
use std::thread;
use std::time::Duration;

const VID: u16 = 0x048d;
const PID: u16 = 0xc693;

fn send_zone_packet(device: &hidapi::HidDevice, zone_start: u8, r: u8, g: u8, b: u8, commit: bool) {
    let mut buf = vec![
        0x04,                             // Command
        0x08,                             // Number of zones
        if commit { 0x01 } else { 0x00 }, // Commit
    ];
    
    for i in 0..8 {
        buf.push(zone_start + i);
        buf.push(0x00);
    }
    
    for _ in 0..8 {
        buf.push(r);
        buf.push(g);
        buf.push(b);
        buf.push(0x01);
    }
    
    println!("Sending packet for zones {}-{} with color ({}, {}, {}), commit={}...", zone_start, zone_start + 7, r, g, b, commit);
    device.send_feature_report(&buf).expect("Failed to send packet");
    thread::sleep(Duration::from_millis(if commit { 10 } else { 5 }));
}

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
    thread::sleep(Duration::from_millis(50)); // give it more time
    
    // Set 0-7 to RED, 8-15 to GREEN, 16-23 to BLUE
    send_zone_packet(&device, 0, 255, 0, 0, false);
    send_zone_packet(&device, 8, 0, 255, 0, false);
    send_zone_packet(&device, 16, 0, 0, 255, true);
    
    println!("Packets sent! Sleeping 5 seconds...");
    thread::sleep(Duration::from_secs(5));

    println!("Enabling autonomous mode (giving back control)...");
    let auto_on = vec![0x06, 0x01];
    device.send_feature_report(&auto_on).expect("Failed to enable autonomous mode");
    thread::sleep(Duration::from_millis(10));

    println!("Done!");
}
