# Lenovo 24-Zone RGB Controller

[![WinUI 3 / .NET 8 / Rust](https://skillicons.dev/icons?i=dotnet,rust)](https://learn.microsoft.com/en-us/windows/apps/winui/winui3/)

A lightweight, native high-performance Windows controller for managing 24 independent RGB zones on Lenovo devices. Powered by a high-performance native architecture using a Rust DLL backend (`rgb_backend.dll`) and a WinUI 3 (C# / .NET 8) desktop interface.

## ✨ Features

- **Granular Control:** 24 RGB zones with independent control.
- **High Performance:** Low-latency updates for smooth animations up to 60 FPS, driven directly by a Rust backend with frame rendering synced through a high-frequency FFI callback.
- **Minimal Footprint:** Replaced the previous WebView2/Chromium frontend with a native XAML compositor, reducing memory usage from ~200MB to ~40MB and start times to <500ms.
- **Modular Architecture:** Build and integrate new custom lighting presets in Rust with ease.
- **System Tray Integration:** Run in background mode, minimize to tray on close, and trigger menu options.
- **Global Hotkey:** Configurable system-wide hotkey to cycle presets instantly.
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
- **Typing Reactive:** Typing Rainbow Ripple

---

## 🚀 Quick Start — Development (Windows)

### Prerequisites
- [.NET 8 SDK](https://dotnet.microsoft.com/download/dotnet/8.0)
- [Rust & Cargo](https://rustup.rs/) (MSVC toolchain)
- Visual Studio build tools / Windows App SDK workload

### Building and Running
The repository includes a PowerShell script `build.ps1` to compile both the Rust DLL and the C# app:
```powershell
powershell -ExecutionPolicy Bypass -File build.ps1
```

Once the build is complete, you can run the native executable:
```path
RGBController\bin\x86\Release\net8.0-windows10.0.26100.0\win-x86\RGBController.exe
```

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

- **Frontend (UI):** `RGBController/` — Built with WinUI 3 (XAML/C#) and .NET 8.
  - *Main Shell:* [MainWindow.xaml](file:///d:/Projects/LOQ-24-zone-RGB-controller/RGBController/MainWindow.xaml)
  - *Visual Canvas:* [HomePage.xaml](file:///d:/Projects/LOQ-24-zone-RGB-controller/RGBController/Pages/HomePage.xaml) using a Win2D `CanvasControl` with blur.
  - *Interop Layer:* [RgbInterop.cs](file:///d:/Projects/LOQ-24-zone-RGB-controller/RGBController/Interop/RgbInterop.cs) for P/Invoke bindings.
- **Backend (Rust):** `rust-backend/` — Manages the hardware driver, effect loops, and preset modules.
  - *FFI DLL entry point:* `rust-backend/src/lib.rs` (compiles to `rgb_backend.dll`).
  - *Hardware Driver:* `rust-backend/src/led_driver.rs`
  - *Effect Protocols:* `rust-backend/src/presets/`
- **Build Output:** `RGBController/bin/x86/Release/net8.0-windows10.0.26100.0/win-x86/`

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
   In the `get_available_presets()` function inside `presets/mod.rs`, add your `PresetMetadata` config block. This will automatically expose your effect to the C# UI along with any adjustable parameters (Speed, Density, Size, Colors).
4. **Hook into the Runner:**
   In `rust-backend/src/lib.rs`, import your effect module and append it to the `match preset_name_lc` arm inside the `rgb_set_preset` function.

---

## 📜 License

This project includes a `LICENSE` file in the repository root — please follow and respect the terms provided in that file.

---
*Created by the project contributors. Designed for ultimate keyboard control without the system bloat.*
