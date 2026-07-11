use std::os::raw::{c_int, c_ulong};
use std::os::unix::io::AsRawFd;
use std::path::Path;

const USBDEVFS_RESET: c_ulong = 0x5514;

extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
}

fn main() {
    println!("Locating ITE device in Upgrade Mode (048d:89db)...");
    
    // Scan sysfs to find the bus and device number
    let mut target_path = None;
    if let Ok(entries) = std::fs::read_dir("/sys/bus/usb/devices") {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                let vid_path = path.join("idVendor");
                let pid_path = path.join("idProduct");
                let bus_path = path.join("busnum");
                let dev_path = path.join("devnum");
                
                if vid_path.exists() && pid_path.exists() {
                    let vid = std::fs::read_to_string(vid_path).unwrap_or_default().trim().to_string();
                    let pid = std::fs::read_to_string(pid_path).unwrap_or_default().trim().to_string();
                    
                    if vid == "048d" && (pid == "89db" || pid == "c693") {
                        let bus = std::fs::read_to_string(bus_path).unwrap_or_default().trim().to_string();
                        let dev = std::fs::read_to_string(dev_path).unwrap_or_default().trim().to_string();
                        
                        // Parse bus and dev numbers to format like /dev/bus/usb/001/007
                        if let (Ok(bus_num), Ok(dev_num)) = (bus.parse::<u32>(), dev.parse::<u32>()) {
                            let usb_path = format!("/dev/bus/usb/{:03}/{:03}", bus_num, dev_num);
                            println!("Found device at: {} (VID: {}, PID: {})", usb_path, vid, pid);
                            target_path = Some(usb_path);
                            break;
                        }
                    }
                }
            }
        }
    }
    
    let path_str = match target_path {
        Some(p) => p,
        None => {
            eprintln!("Error: No ITE device (048d:89db or 048d:c693) found.");
            return;
        }
    };
    
    let path = Path::new(&path_str);
    println!("Opening USB device node...");
    // Open in write mode to allow ioctl
    let file = match std::fs::OpenOptions::new().write(true).open(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error opening device: {}. (You may need to run this with sudo or appropriate permissions)", e);
            return;
        }
    };
    
    println!("Sending USBDEVFS_RESET ioctl...");
    unsafe {
        let fd = file.as_raw_fd();
        let res = ioctl(fd, USBDEVFS_RESET, 0);
        if res < 0 {
            let err = std::io::Error::last_os_error();
            eprintln!("Failed to reset USB device: {}", err);
        } else {
            println!("Successfully sent USB reset command! Check 'lsusb' to see if PID has changed back to c693.");
        }
    }
}
