use xcap::Monitor;

fn main() {
    println!("Querying monitors via xcap...");
    match Monitor::all() {
        Ok(monitors) => {
            println!("Found {} monitors:", monitors.len());
            for (i, m) in monitors.iter().enumerate() {
                println!("Monitor {}: Name: {:?}, Width: {:?}, Height: {:?}", i, m.name(), m.width(), m.height());
                println!("Attempting to capture image...");
                match m.capture_image() {
                    Ok(img) => {
                        println!("Success! Captured image dimensions: {}x{}", img.width(), img.height());
                        // Print some pixel values to make sure it's not all black
                        if img.width() > 10 && img.height() > 10 {
                            let p = img.get_pixel(10, 10);
                            println!("Pixel at 10,10: {:?}", p);
                        }
                    }
                    Err(e) => {
                        eprintln!("Capture failed: {:?}", e);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Error querying monitors: {:?}", e);
        }
    }
}
