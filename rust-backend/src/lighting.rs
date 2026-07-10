/// Lighting priority management.
///
/// On Windows this manipulates the Windows Dynamic Lighting registry to set
/// priority. On Linux this concept does not exist, so all functions are no-ops.

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use std::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn enable_windows_lighting() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        let script = r#"
            $lightingPath = "HKCU:\Software\Microsoft\Lighting"
            if (-not (Test-path $lightingPath)) {
                New-Item -Path $lightingPath -Force | Out-Null
            }
            Set-ItemProperty -Path $lightingPath -Name "IsEnabled" -Value 1 -Type DWord -Force
            Write-Output "Enabled"
        "#;
        let output = Command::new("powershell")
            .args(["-ExecutionPolicy", "Bypass", "-NoProfile", "-NonInteractive", "-Command", script])
            .creation_flags(CREATE_NO_WINDOW)
            .output()?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into());
        }
    }
    Ok(())
}

pub fn disable_windows_lighting() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        let script = r#"
            $lightingPath = "HKCU:\Software\Microsoft\Lighting"
            if (Test-Path $lightingPath) {
                Set-ItemProperty -Path $lightingPath -Name "IsEnabled" -Value 0 -Type DWord -Force
            }
            Write-Output "Disabled"
        "#;
        let output = Command::new("powershell")
            .args(["-ExecutionPolicy", "Bypass", "-NoProfile", "-NonInteractive",
                   "-WindowStyle", "Hidden", "-Command", script])
            .creation_flags(CREATE_NO_WINDOW)
            .output()?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into());
        }
    }
    Ok(())
}

pub fn is_windows_lighting_enabled() -> bool {
    #[cfg(windows)]
    {
        let script = r#"
            $lightingPath = "HKCU:\Software\Microsoft\Lighting"
            if (Test-Path $lightingPath) {
                $props = Get-ItemProperty -Path $lightingPath -ErrorAction SilentlyContinue
                if ($props.IsEnabled -eq 1) { Write-Output "enabled" }
            }
        "#;
        let output = Command::new("powershell")
            .args(["-ExecutionPolicy", "Bypass", "-NoProfile", "-NonInteractive",
                   "-WindowStyle", "Hidden", "-Command", script])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        if let Ok(out) = output {
            return String::from_utf8_lossy(&out.stdout).trim() == "enabled";
        }
    }
    false
}

pub fn set_windows_lighting_on_top() -> Result<(), Box<dyn std::error::Error>> {
    // No-op on Linux — no concept of "Dynamic Lighting" providers.
    Ok(())
}
