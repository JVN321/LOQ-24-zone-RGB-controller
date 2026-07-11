# Lenovo 24-Zone RGB Controller (Linux Port)

[![Linux / Rust](https://skillicons.dev/icons?i=linux,rust)](https://github.com/JVN321/LOQ-24-zone-RGB-controller)

A lightweight, native high-performance Linux port/controller for managing 24 independent RGB zones on Lenovo LOQ devices. This project features a high-performance background daemon and a premium Axum-powered Web Server with a responsive, glassmorphic UI.

## ✨ Features

- **Granular Control:** 24 RGB zones with independent control.
- **High Performance:** Low-latency updates for smooth animations up to 60 FPS, driven directly by a Rust backend.
- **Minimal Footprint:** Replaced system bloat with a native daemon, reducing memory usage and starting instantly.
- **Modular Architecture:** Build and integrate new custom lighting presets in Rust with ease.
- **Direct USB HID Control:** Speaks natively to the hardware backend.

## 🎨 Supported Effects

- Static Color
- CPU-Mem-GPU usage status
- Screen Ambiance *(Reactive)*
- Color Breath
- Pulse Center
- Horse Color & Horse Cycle
- Ferrari RPM
- Rainbow Breath, Rainbow Cycle, & Rainbow Wave
- ColorWheelEffect & Color sweep
- Aurora
- Color Scan
- Sparkle
- Nebula
- Chromatic Breath
- **Audio Reactive:** Audio Sparkle, Audio Sparkle Rainbow, Audio Sparkle Media, Audio Ripple
- **Typing Reactive:** Typing Rainbow Ripple, Keyboard Wave *(glowing rainbow wave with water ripples)*
- **Multi-Effect Layering:** Stack, layer, and mix multiple effects together with customizable opacity, intensity, and priority levels.

---

## 🚀 Quick Start — Development (Linux)

The Linux port provides a lightweight background daemon and Axum-powered Web Server with a responsive, glassmorphic UI to control the keyboard zones and settings.

### 📋 Prerequisites & Packages

Ensure the following system dependencies are installed before building.

#### 📦 Fedora (Workstation/Silverblue/etc.)
```bash
sudo dnf install -y \
    alsa-lib-devel \
    libX11-devel \
    libXi-devel \
    libXrandr-devel \
    hidapi-devel \
    pkgconf-pkg-config \
    gcc \
    wayland-devel \
    libxkbcommon-devel \
    mesa-libEGL-devel \
    mesa-libgbm-devel \
    pipewire-devel \
    dbus-devel \
    clang
```

#### 📦 Debian / Ubuntu
```bash
sudo apt-get update && sudo apt-get install -y \
    libasound2-dev \
    libx11-dev \
    libxi-dev \
    libxrandr-dev \
    libhidapi-dev \
    pkg-config \
    build-essential \
    libwayland-dev \
    libxkbcommon-dev \
    libegl1-mesa-dev \
    libgbm-dev \
    libpipewire-0.3-dev \
    libdbus-1-dev \
    clang
```

### 🔌 Hardware Permissions (udev)

To allow the server to write to the USB HID interface without running as `root`, copy the udev rule and configure groups:

```bash
# 1. Copy the udev rule
sudo cp 99-loq-rgb.rules /etc/udev/rules.d/

# 2. Reload and trigger rules
sudo udevadm control --reload-rules
sudo udevadm trigger

# 3. Add your user to the plugdev (or input) group
sudo usermod -aG plugdev $USER
sudo usermod -aG input $USER

# Log out and log back in (or use 'newgrp plugdev' and 'newgrp input')
```

### 🛠️ Building and Running

1. **Compile Backend and Server:**
   ```bash
   ./build.sh
   ```
2. **Start the Controller Daemon:**
   ```bash
   ./target/release/rgb-server
   ```
3. **Open the Controller UI:**
   Navigate to [http://127.0.0.1:7070](http://127.0.0.1:7070) in your web browser.

---

## 🏗️ Architecture Overview

- **Frontend (UI):** Embedded HTML/CSS/JS page (`rgb-server/static/index.html`) using modern WebSockets and JSON REST API bindings for real-time visualization and settings adjustment.
- **Server:** `rgb-server/` — An Axum web server hosting the REST endpoints and WebSocket broadcast hubs.
- **Backend (Rust):** `rust-backend/` — Core Rust crate managing the Linux HID hardware drivers, reactive samplers (PipeWire, xcap/DBus, evdev), and preset modules.

---

## 🛠️ Creating Custom Effects

You can implement customized static or dynamic effects using Rust. Use the `rust-backend/src/effects.rs` interface template to get started.

1. **Create the Effect File:**
   Create a new file under `rust-backend/src/presets/` (e.g., `my_custom_effect.rs`) and implement the `Effect` trait.
2. **Register the Module:**
   Open `rust-backend/src/presets/mod.rs` and expose your module:
   ```rust
   pub mod my_custom_effect;
   ```
3. **Define Preset Metadata:**
   In the `get_available_presets()` function inside `presets/mod.rs`, add your `PresetMetadata` config block. This will automatically expose your effect to the web UI along with any adjustable parameters (Speed, Density, Size, Colors).
4. **Hook into the Runner:**
   Open `rust-backend/src/lib.rs` and append your effect constructor mapping to the `build_effect` helper logic.

---

## 📜 License

This project includes a `LICENSE` file in the repository root — please follow and respect the terms provided in that file.

---
*Created by the project contributors. Designed for ultimate keyboard control without the system bloat.*
