use hidapi::HidApi;

fn main() {
    println!("Listing all HID devices:");
    match HidApi::new() {
        Ok(api) => {
            for device in api.device_list() {
                if device.vendor_id() == 0x048d {
                    println!(
                        "Vendor ID: {:04x}, Product ID: {:04x}, Path: {:?}, Interface: {}, Usage Page: 0x{:04x}, Usage: 0x{:04x}",
                        device.vendor_id(),
                        device.product_id(),
                        device.path(),
                        device.interface_number(),
                        device.usage_page(),
                        device.usage(),
                    );
                }
            }
        }
        Err(e) => {
            eprintln!("Error initializing HidApi: {}", e);
        }
    }
}
