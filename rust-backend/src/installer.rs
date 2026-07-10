/// Startup task management.
///
/// On Windows: creates a Windows Task Scheduler task.
/// On Linux: creates a systemd user service.

#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;
#[allow(dead_code)]
const TASK_NAME: &str = "SetWindowsLightingOnTop";

pub fn create_startup_task(delay_seconds: u32) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        let script_content = r#"
function Set-WindowsOnTop {
    param($path)
    if (Test-Path $path) {
        $props = Get-ItemProperty -Path $path -ErrorAction SilentlyContinue
        if ($props) {
            $provider1 = $props.'1'
            $provider2 = $props.'2'
            if ($provider1 -ne "WindowsLighting" -and $provider2 -eq "WindowsLighting") {
                Set-ItemProperty -Path $path -Name "1" -Value $provider2 -Force
                Set-ItemProperty -Path $path -Name "2" -Value $provider1 -Force
            }
        }
    }
}
$providersPath = "HKCU:\Software\Microsoft\Lighting\Providers"
$devicesPath = "HKCU:\Software\Microsoft\Lighting\Devices"
Set-WindowsOnTop -path $providersPath
if (Test-Path $devicesPath) {
    Get-ChildItem -Path $devicesPath -Recurse -ErrorAction SilentlyContinue |
        Where-Object { $_.PSChildName -eq "Providers" } |
        ForEach-Object { Set-WindowsOnTop -path $_.PSPath }
}
"#;
        let appdata = std::env::var("APPDATA")?;
        let script_path = format!("{}\\SetWindowsLightingOnTop.ps1", appdata);
        std::fs::write(&script_path, script_content)?;

        let task_script = format!(r#"
            $taskName = "{}"
            $scriptPath = "{}"
            $delaySeconds = {}
            Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue
            $action = New-ScheduledTaskAction -Execute "powershell.exe" -Argument "-ExecutionPolicy Bypass -NoProfile -WindowStyle Hidden -File `"$scriptPath`""
            $trigger1 = New-ScheduledTaskTrigger -AtLogOn
            $trigger1.Delay = "PT$($delaySeconds)S"
            $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -Hidden
            Register-ScheduledTask -TaskName $taskName -Action $action -Trigger $trigger1 -Settings $settings -Force | Out-Null
            Write-Output "Success"
        "#, TASK_NAME, script_path.replace("\\", "\\\\"), delay_seconds);

        let output = Command::new("powershell")
            .args(["-ExecutionPolicy", "Bypass", "-NoProfile", "-NonInteractive",
                   "-WindowStyle", "Hidden", "-Command", &task_script])
            .creation_flags(CREATE_NO_WINDOW)
            .output()?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into());
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let exe = std::env::current_exe()
            .unwrap_or_else(|_| std::path::PathBuf::from("/usr/local/bin/loq-rgb"));

        let service = format!(
            "[Unit]\nDescription=LOQ RGB Controller\nAfter=graphical-session.target\n\n\
             [Service]\nType=simple\nExecStartPre=/bin/sleep {}\nExecStart={}\nRestart=on-failure\n\n\
             [Install]\nWantedBy=default.target\n",
            delay_seconds,
            exe.display()
        );

        let config_dir = std::env::var("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_default();
                std::path::PathBuf::from(home).join(".config")
            });
        let service_dir = config_dir.join("systemd/user");
        std::fs::create_dir_all(&service_dir)?;
        let service_path = service_dir.join("loq-rgb.service");
        std::fs::write(&service_path, service)?;

        Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status()?;
        Command::new("systemctl")
            .args(["--user", "enable", "loq-rgb.service"])
            .status()?;
    }

    Ok(())
}

pub fn remove_startup_task() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        let script = format!(r#"
            Unregister-ScheduledTask -TaskName "{}" -Confirm:$false -ErrorAction SilentlyContinue
            $scriptPath = "$env:APPDATA\SetWindowsLightingOnTop.ps1"
            if (Test-Path $scriptPath) {{ Remove-Item $scriptPath -Force }}
            Write-Output "Success"
        "#, TASK_NAME);
        let output = Command::new("powershell")
            .args(["-ExecutionPolicy", "Bypass", "-NoProfile", "-NonInteractive",
                   "-WindowStyle", "Hidden", "-Command", &script])
            .creation_flags(CREATE_NO_WINDOW)
            .output()?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into());
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new("systemctl")
            .args(["--user", "disable", "loq-rgb.service"])
            .status()
            .ok();

        let config_dir = std::env::var("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_default();
                std::path::PathBuf::from(home).join(".config")
            });
        let service_path = config_dir.join("systemd/user/loq-rgb.service");
        if service_path.exists() {
            std::fs::remove_file(service_path)?;
        }
    }

    Ok(())
}

pub fn is_startup_task_installed() -> bool {
    #[cfg(windows)]
    {
        let output = Command::new("schtasks")
            .args(["/query", "/tn", TASK_NAME])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        return output.map(|o| o.status.success()).unwrap_or(false);
    }

    #[cfg(not(target_os = "windows"))]
    {
        let config_dir = std::env::var("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_default();
                std::path::PathBuf::from(home).join(".config")
            });
        config_dir.join("systemd/user/loq-rgb.service").exists()
    }
}