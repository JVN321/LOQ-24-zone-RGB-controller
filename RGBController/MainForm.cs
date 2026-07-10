#pragma warning disable CS0649

using System;
using System.Drawing;
using System.IO;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Windows.Forms;
using Microsoft.Win32;
using RGBController.Interop;
using RGBController.Controls;
using AppSettings = RGBController.Models.AppSettings;
using Color = System.Drawing.Color;

namespace RGBController
{
    public partial class MainForm : Form
    {
        [DllImport("dwmapi.dll")]
        private static extern int DwmSetWindowAttribute(IntPtr hwnd, int attr, ref int attrValue, int attrSize);

        [DllImport("user32.dll")]
        private static extern bool RegisterHotKey(IntPtr hWnd, int id, uint fsModifiers, uint vk);

        [DllImport("user32.dll")]
        private static extern bool UnregisterHotKey(IntPtr hWnd, int id);

        private const int DWMWA_USE_IMMERSIVE_DARK_MODE = 20;
        private const int WM_GETMINMAXINFO = 0x0024;
        private const int WM_HOTKEY = 0x0312;

        private struct MINMAXINFO
        {
            public Point ptReserved;
            public Point ptMaxSize;
            public Point ptMaxPosition;
            public Point ptMinTrackSize;
            public Point ptMaxTrackSize;
        }

        private bool reallyClose = false;
        private bool isHotkeyRegistered = false;
        private bool startMinimized = false;
        private bool isFirstShow = true;

        // Active page panels
        private HomePanel? homePanel;
        private HardwarePanel? hardwarePanel;
        private ConsolePanel? consolePanel;
        private SettingsPanel? settingsPanel;

        public event Action? OnPresetCycled;

        public MainForm(bool startMinimized = false)
        {
            this.startMinimized = startMinimized;
            InitializeComponent();
            ApplyTheme();
            LoadAppIcon();
            
            // Default page
            SwitchToPanel("Home");

            // Load initial brightness and settings
            LoadSettingsAndRegisterHotkey();

            // Listen for sleep/resume to recover from stale HID handles
            SystemEvents.PowerModeChanged += OnPowerModeChanged;
        }

        protected override void SetVisibleCore(bool value)
        {
            if (isFirstShow && startMinimized)
            {
                isFirstShow = false;
                value = false;
                if (!this.IsHandleCreated)
                {
                    CreateHandle();
                }
            }
            base.SetVisibleCore(value);
        }

        private void ApplyTheme()
        {
            this.BackColor = Theme.Background;
            this.ForeColor = Theme.TextPrimary;

            // Immersive dark mode for title bar
            int darkMode = 1;
            DwmSetWindowAttribute(this.Handle, DWMWA_USE_IMMERSIVE_DARK_MODE, ref darkMode, sizeof(int));

            // Sidebar styling
            this.sidebarPanel.BackColor = Theme.Card;
            
            // Brand styling
            this.brandPanel.BackColor = Theme.Card;
            Theme.StyleLabel(this.brandLabel, Theme.FontTitle, Theme.TextPrimary);

            // Nav buttons styling
            StyleNavButton(this.btnHome);
            StyleNavButton(this.btnHardware);
            StyleNavButton(this.btnConsole);
            StyleNavButton(this.btnSettings);

            // Footer styling
            this.footerPanel.BackColor = Theme.Card;
            Theme.StyleLabel(this.brightnessLabel, Theme.FontMonospace, Theme.TextSecondary);
            this.brightnessTrackBar.BackColor = Theme.Card;
        }

        private void StyleNavButton(Button btn)
        {
            btn.FlatStyle = FlatStyle.Flat;
            btn.BackColor = Theme.Card;
            btn.ForeColor = Theme.TextSecondary;
            btn.Font = Theme.FontHeader;
            btn.FlatAppearance.BorderSize = 0;
            btn.FlatAppearance.MouseOverBackColor = Color.FromArgb(20, 20, 23);
            btn.FlatAppearance.MouseDownBackColor = Color.FromArgb(12, 12, 14);
            btn.TextAlign = ContentAlignment.MiddleLeft;
            btn.Padding = new Padding(24, 0, 0, 0);
        }

        private void LoadAppIcon()
        {
            string iconPath = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "Assets", "AppIcon.ico");
            if (File.Exists(iconPath))
            {
                try
                {
                    Icon appIcon = new Icon(iconPath);
                    this.Icon = appIcon;
                    this.trayIcon.Icon = appIcon;
                }
                catch
                {
                    // Fallback
                }
            }
            else
            {
                // Try relative to exe
                iconPath = Path.Combine(Directory.GetCurrentDirectory(), "Assets", "AppIcon.ico");
                if (File.Exists(iconPath))
                {
                    try
                    {
                        Icon appIcon = new Icon(iconPath);
                        this.Icon = appIcon;
                        this.trayIcon.Icon = appIcon;
                    }
                    catch { }
                }
            }
        }

        private void BtnNavigation_Click(object sender, EventArgs e)
        {
            if (sender is Button btn)
            {
                SwitchToPanel(btn.Text);
            }
        }

        public void SwitchToPanel(string pageName)
        {
            // Highlight active button
            ResetNavButtons();
            
            UserControl targetPanel;

            switch (pageName)
            {
                case "Home":
                    this.btnHome.ForeColor = Theme.TextPrimary;
                    this.btnHome.BackColor = Color.FromArgb(18, 18, 20);
                    if (homePanel == null || homePanel.IsDisposed)
                    {
                        homePanel = new HomePanel(this);
                    }
                    targetPanel = homePanel;
                    break;
                case "Hardware":
                    this.btnHardware.ForeColor = Theme.TextPrimary;
                    this.btnHardware.BackColor = Color.FromArgb(18, 18, 20);
                    if (hardwarePanel == null || hardwarePanel.IsDisposed)
                    {
                        hardwarePanel = new HardwarePanel();
                    }
                    targetPanel = hardwarePanel;
                    break;
                case "Console":
                    this.btnConsole.ForeColor = Theme.TextPrimary;
                    this.btnConsole.BackColor = Color.FromArgb(18, 18, 20);
                    if (consolePanel == null || consolePanel.IsDisposed)
                    {
                        consolePanel = new ConsolePanel();
                    }
                    targetPanel = consolePanel;
                    break;
                case "Settings":
                    this.btnSettings.ForeColor = Theme.TextPrimary;
                    this.btnSettings.BackColor = Color.FromArgb(18, 18, 20);
                    if (settingsPanel == null || settingsPanel.IsDisposed)
                    {
                        settingsPanel = new SettingsPanel(this);
                    }
                    targetPanel = settingsPanel;
                    break;
                default:
                    return;
            }

            this.contentPanel.Controls.Clear();
            targetPanel.Dock = DockStyle.Fill;
            this.contentPanel.Controls.Add(targetPanel);
            
            if (targetPanel is HomePanel hp)
            {
                hp.ReloadPresetData();
            }
        }

        private void ResetNavButtons()
        {
            this.btnHome.ForeColor = Theme.TextSecondary;
            this.btnHome.BackColor = Theme.Card;
            this.btnHardware.ForeColor = Theme.TextSecondary;
            this.btnHardware.BackColor = Theme.Card;
            this.btnConsole.ForeColor = Theme.TextSecondary;
            this.btnConsole.BackColor = Theme.Card;
            this.btnSettings.ForeColor = Theme.TextSecondary;
            this.btnSettings.BackColor = Theme.Card;
        }

        private void BrightnessTrackBar_Scroll(object sender, EventArgs e)
        {
            float brightness = brightnessTrackBar.Value / 100f;
            RgbInterop.rgb_set_brightness(brightness);
            brightnessLabel.Text = $"BRIGHTNESS: {brightnessTrackBar.Value}%";
            
            // Save brightness in settings
            try
            {
                string settingsJson = RgbInterop.GetSettings();
                if (!string.IsNullOrEmpty(settingsJson))
                {
                    var settings = JsonSerializer.Deserialize<AppSettings>(settingsJson);
                    if (settings != null)
                    {
                        settings.BrightnessLevel = brightness;
                        string updatedJson = JsonSerializer.Serialize(settings);
                        RgbInterop.SaveSettings(updatedJson);
                    }
                }
            }
            catch { }
        }

        public void LoadSettingsAndRegisterHotkey()
        {
            try
            {
                string settingsJson = RgbInterop.GetSettings();
                if (!string.IsNullOrEmpty(settingsJson))
                {
                    var settings = JsonSerializer.Deserialize<AppSettings>(settingsJson);
                    if (settings != null)
                    {
                        // Update brightness UI
                        int brightnessVal = (int)(settings.BrightnessLevel * 100);
                        brightnessTrackBar.Value = Math.Clamp(brightnessVal, 0, 100);
                        brightnessLabel.Text = $"BRIGHTNESS: {brightnessTrackBar.Value}%";

                        // Register global hotkey
                        RegisterGlobalHotkey(settings.PresetCycleShortcut);
                    }
                }
            }
            catch { }
        }

        public void UnregisterCurrentHotkey()
        {
            if (isHotkeyRegistered)
            {
                UnregisterHotKey(this.Handle, 1);
                isHotkeyRegistered = false;
            }
        }

        private void RegisterGlobalHotkey(string? shortcut)
        {
            // Unregister first
            UnregisterCurrentHotkey();

            if (string.IsNullOrEmpty(shortcut)) return;

            if (ParseShortcut(shortcut, out uint modifiers, out uint vk))
            {
                isHotkeyRegistered = RegisterHotKey(this.Handle, 1, modifiers, vk);
            }
        }

        private bool ParseShortcut(string shortcut, out uint modifiers, out uint vk)
        {
            modifiers = 0;
            vk = 0;
            if (string.IsNullOrEmpty(shortcut)) return false;

            string[] parts = shortcut.Split('+');
            foreach (string part in parts)
            {
                string clean = part.Trim().ToLowerInvariant();
                if (clean == "ctrl" || clean == "control" || clean == "commandorcontrol")
                {
                    modifiers |= 0x0002; // MOD_CONTROL
                }
                else if (clean == "shift")
                {
                    modifiers |= 0x0004; // MOD_SHIFT
                }
                else if (clean == "alt")
                {
                    modifiers |= 0x0001; // MOD_ALT
                }
                else if (clean == "win" || clean == "windows" || clean == "cmd")
                {
                    modifiers |= 0x0008; // MOD_WIN
                }
                else
                {
                    if (Enum.TryParse<Keys>(part, true, out Keys key))
                    {
                        vk = (uint)key;
                    }
                    else if (clean == "pageup") vk = (uint)Keys.PageUp;
                    else if (clean == "pagedown") vk = (uint)Keys.PageDown;
                    else if (clean.Length == 1)
                    {
                        char c = clean[0];
                        if (c >= 'a' && c <= 'z')
                        {
                            vk = (uint)(Keys.A + (c - 'a'));
                        }
                        else if (c >= '0' && c <= '9')
                        {
                            vk = (uint)(Keys.D0 + (c - '0'));
                        }
                    }
                }
            }
            return vk != 0;
        }

        private System.Collections.Generic.List<RGBController.Models.PresetMetadata>? _presetsCache;

        private string GetPresetDisplayName(string name)
        {
            if (_presetsCache == null)
            {
                try
                {
                    string json = RgbInterop.GetPresetMetadata();
                    _presetsCache = JsonSerializer.Deserialize<System.Collections.Generic.List<RGBController.Models.PresetMetadata>>(json);
                }
                catch { }
            }

            if (_presetsCache != null)
            {
                var preset = _presetsCache.Find(p => p.Name.Equals(name, StringComparison.OrdinalIgnoreCase));
                if (preset != null)
                {
                    return preset.DisplayName;
                }
            }
            return name;
        }

        private void CyclePresetAndNotify()
        {
            string nextPreset = RgbInterop.CyclePreset();
            if (!string.IsNullOrEmpty(nextPreset) && !nextPreset.StartsWith("Error"))
            {
                SettingsManager.SaveActivePreset(nextPreset);
                string displayName = GetPresetDisplayName(nextPreset);
                OsdPopup.Show(displayName, this);
            }
            
            // Notify UI
            if (OnPresetCycled != null)
            {
                if (this.InvokeRequired)
                {
                    this.BeginInvoke(OnPresetCycled);
                }
                else
                {
                    OnPresetCycled.Invoke();
                }
            }
        }

        protected override void WndProc(ref Message m)
        {
            if (m.Msg == WM_GETMINMAXINFO)
            {
                MINMAXINFO mmi = (MINMAXINFO)Marshal.PtrToStructure(m.LParam, typeof(MINMAXINFO))!;
                mmi.ptMinTrackSize.X = 740;
                mmi.ptMinTrackSize.Y = 540;
                Marshal.StructureToPtr(mmi, m.LParam, true);
                return;
            }
            else if (m.Msg == WM_HOTKEY)
            {
                if (m.WParam.ToInt32() == 1)
                {
                    CyclePresetAndNotify();
                }
            }
            base.WndProc(ref m);
        }

        protected override void OnFormClosing(FormClosingEventArgs e)
        {
            if (!reallyClose && e.CloseReason == CloseReason.UserClosing)
            {
                e.Cancel = true;
                this.Hide();
                trayIcon.ShowBalloonTip(2000, "LOQ RGB Controller", "App minimized to system tray. Double click icon to restore.", ToolTipIcon.Info);
            }
            else
            {
                // --- Full cleanup BEFORE base.OnFormClosing ---
                // This prevents Application.OpenForms from being modified during enumeration.

                SystemEvents.PowerModeChanged -= OnPowerModeChanged;

                // Stop the frame callback so the native backend stops invoking into managed code
                try { RgbInterop.rgb_start_frame_callback(null); } catch { }

                // Dismiss any active OSD popup (it's a Form in Application.OpenForms)
                OsdPopup.DismissCurrent();

                // Clear content panel and dispose child panels
                this.contentPanel.Controls.Clear();
                homePanel?.Dispose();
                hardwarePanel?.Dispose();
                consolePanel?.Dispose();
                settingsPanel?.Dispose();
                homePanel = null;
                hardwarePanel = null;
                consolePanel = null;
                settingsPanel = null;

                // Unregister global hotkey
                UnregisterCurrentHotkey();

                trayIcon.Visible = false;
                base.OnFormClosing(e);
            }
        }

        private void TrayIcon_DoubleClick(object sender, EventArgs e)
        {
            ShowForm();
        }

        private void MenuShow_Click(object sender, EventArgs e)
        {
            ShowForm();
        }

        private void MenuExit_Click(object sender, EventArgs e)
        {
            reallyClose = true;
            this.Close();
            Application.Exit();
        }

        private void ShowForm()
        {
            this.Show();
            this.WindowState = FormWindowState.Normal;
            this.Activate();
        }

        private void OnPowerModeChanged(object sender, PowerModeChangedEventArgs e)
        {
            if (e.Mode == PowerModes.Resume)
            {
                // The system just woke from sleep/hibernate.
                // The Rust backend detects stale HID handles and reconnects automatically,
                // but we also need to re-register the frame callback on the C# side
                // because the UI thread may have had queued BeginInvoke calls that went stale.
                try
                {
                    // Re-register the frame callback so the HomePanel visualizer resumes cleanly
                    if (homePanel != null && !homePanel.IsDisposed)
                    {
                        // Force the HomePanel to re-setup its frame callback
                        this.BeginInvoke(new Action(() =>
                        {
                            try
                            {
                                homePanel.ReloadPresetData();
                            }
                            catch { }
                        }));
                    }
                }
                catch { }
            }
        }
    }
}
