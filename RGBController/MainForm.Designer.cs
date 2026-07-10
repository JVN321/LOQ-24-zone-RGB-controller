namespace RGBController
{
    partial class MainForm
    {
        private System.ComponentModel.IContainer components = null;
        private System.Windows.Forms.Panel sidebarPanel;
        private System.Windows.Forms.Panel contentPanel;
        private System.Windows.Forms.Panel brandPanel;
        private System.Windows.Forms.Label brandLabel;
        private System.Windows.Forms.Button btnHome;
        private System.Windows.Forms.Button btnHardware;
        private System.Windows.Forms.Button btnConsole;
        private System.Windows.Forms.Button btnSettings;
        private System.Windows.Forms.Panel footerPanel;
        private System.Windows.Forms.Label brightnessLabel;
        private System.Windows.Forms.TrackBar brightnessTrackBar;
        private System.Windows.Forms.NotifyIcon trayIcon;
        private System.Windows.Forms.ContextMenuStrip trayMenu;
        private System.Windows.Forms.ToolStripMenuItem menuShow;
        private System.Windows.Forms.ToolStripMenuItem menuExit;

        protected override void Dispose(bool disposing)
        {
            if (disposing && (components != null))
            {
                components.Dispose();
            }
            base.Dispose(disposing);
        }

        private void InitializeComponent()
        {
            this.components = new System.ComponentModel.Container();
            this.sidebarPanel = new System.Windows.Forms.Panel();
            this.contentPanel = new System.Windows.Forms.Panel();
            this.brandPanel = new System.Windows.Forms.Panel();
            this.brandLabel = new System.Windows.Forms.Label();
            this.btnHome = new System.Windows.Forms.Button();
            this.btnHardware = new System.Windows.Forms.Button();
            this.btnConsole = new System.Windows.Forms.Button();
            this.btnSettings = new System.Windows.Forms.Button();
            this.footerPanel = new System.Windows.Forms.Panel();
            this.brightnessLabel = new System.Windows.Forms.Label();
            this.brightnessTrackBar = new System.Windows.Forms.TrackBar();
            this.trayIcon = new System.Windows.Forms.NotifyIcon(this.components);
            this.trayMenu = new System.Windows.Forms.ContextMenuStrip(this.components);
            this.menuShow = new System.Windows.Forms.ToolStripMenuItem();
            this.menuExit = new System.Windows.Forms.ToolStripMenuItem();

            this.sidebarPanel.SuspendLayout();
            this.brandPanel.SuspendLayout();
            this.footerPanel.SuspendLayout();
            ((System.ComponentModel.ISupportInitialize)(this.brightnessTrackBar)).BeginInit();
            this.trayMenu.SuspendLayout();
            this.SuspendLayout();

            // 
            // sidebarPanel
            // 
            this.sidebarPanel.Controls.Add(this.btnSettings);
            this.sidebarPanel.Controls.Add(this.btnConsole);
            this.sidebarPanel.Controls.Add(this.btnHardware);
            this.sidebarPanel.Controls.Add(this.btnHome);
            this.sidebarPanel.Controls.Add(this.footerPanel);
            this.sidebarPanel.Controls.Add(this.brandPanel);
            this.sidebarPanel.Dock = System.Windows.Forms.DockStyle.Left;
            this.sidebarPanel.Location = new System.Drawing.Point(0, 0);
            this.sidebarPanel.Name = "sidebarPanel";
            this.sidebarPanel.Size = new System.Drawing.Size(200, 640);
            this.sidebarPanel.TabIndex = 0;

            // 
            // brandPanel
            // 
            this.brandPanel.Controls.Add(this.brandLabel);
            this.brandPanel.Dock = System.Windows.Forms.DockStyle.Top;
            this.brandPanel.Location = new System.Drawing.Point(0, 0);
            this.brandPanel.Name = "brandPanel";
            this.brandPanel.Size = new System.Drawing.Size(200, 70);
            this.brandPanel.TabIndex = 0;

            // 
            // brandLabel
            // 
            this.brandLabel.Dock = System.Windows.Forms.DockStyle.Fill;
            this.brandLabel.Location = new System.Drawing.Point(0, 0);
            this.brandLabel.Name = "brandLabel";
            this.brandLabel.Size = new System.Drawing.Size(200, 70);
            this.brandLabel.TabIndex = 0;
            this.brandLabel.Text = "LEGION LIGHTING";
            this.brandLabel.TextAlign = System.Drawing.ContentAlignment.MiddleCenter;

            // 
            // btnHome
            // 
            this.btnHome.Dock = System.Windows.Forms.DockStyle.Top;
            this.btnHome.Location = new System.Drawing.Point(0, 70);
            this.btnHome.Name = "btnHome";
            this.btnHome.Size = new System.Drawing.Size(200, 48);
            this.btnHome.TabIndex = 1;
            this.btnHome.Text = "Home";
            this.btnHome.UseVisualStyleBackColor = true;
            this.btnHome.Click += new System.EventHandler(this.BtnNavigation_Click);

            // 
            // btnHardware
            // 
            this.btnHardware.Dock = System.Windows.Forms.DockStyle.Top;
            this.btnHardware.Location = new System.Drawing.Point(0, 118);
            this.btnHardware.Name = "btnHardware";
            this.btnHardware.Size = new System.Drawing.Size(200, 48);
            this.btnHardware.TabIndex = 2;
            this.btnHardware.Text = "Hardware";
            this.btnHardware.UseVisualStyleBackColor = true;
            this.btnHardware.Click += new System.EventHandler(this.BtnNavigation_Click);

            // 
            // btnConsole
            // 
            this.btnConsole.Dock = System.Windows.Forms.DockStyle.Top;
            this.btnConsole.Location = new System.Drawing.Point(0, 166);
            this.btnConsole.Name = "btnConsole";
            this.btnConsole.Size = new System.Drawing.Size(200, 48);
            this.btnConsole.TabIndex = 3;
            this.btnConsole.Text = "Console";
            this.btnConsole.UseVisualStyleBackColor = true;
            this.btnConsole.Click += new System.EventHandler(this.BtnNavigation_Click);

            // 
            // btnSettings
            // 
            this.btnSettings.Dock = System.Windows.Forms.DockStyle.Top;
            this.btnSettings.Location = new System.Drawing.Point(0, 214);
            this.btnSettings.Name = "btnSettings";
            this.btnSettings.Size = new System.Drawing.Size(200, 48);
            this.btnSettings.TabIndex = 4;
            this.btnSettings.Text = "Settings";
            this.btnSettings.UseVisualStyleBackColor = true;
            this.btnSettings.Click += new System.EventHandler(this.BtnNavigation_Click);

            // 
            // footerPanel
            // 
            this.footerPanel.Controls.Add(this.brightnessLabel);
            this.footerPanel.Controls.Add(this.brightnessTrackBar);
            this.footerPanel.Dock = System.Windows.Forms.DockStyle.Bottom;
            this.footerPanel.Location = new System.Drawing.Point(0, 540);
            this.footerPanel.Name = "footerPanel";
            this.footerPanel.Size = new System.Drawing.Size(200, 100);
            this.footerPanel.TabIndex = 5;

            // 
            // brightnessLabel
            // 
            this.brightnessLabel.Dock = System.Windows.Forms.DockStyle.Top;
            this.brightnessLabel.Location = new System.Drawing.Point(0, 0);
            this.brightnessLabel.Name = "brightnessLabel";
            this.brightnessLabel.Size = new System.Drawing.Size(200, 30);
            this.brightnessLabel.TabIndex = 0;
            this.brightnessLabel.Text = "BRIGHTNESS: 50%";
            this.brightnessLabel.TextAlign = System.Drawing.ContentAlignment.MiddleCenter;

            // 
            // brightnessTrackBar
            // 
            this.brightnessTrackBar.Dock = System.Windows.Forms.DockStyle.Bottom;
            this.brightnessTrackBar.Location = new System.Drawing.Point(0, 40);
            this.brightnessTrackBar.Maximum = 100;
            this.brightnessTrackBar.Name = "brightnessTrackBar";
            this.brightnessTrackBar.Size = new System.Drawing.Size(200, 45);
            this.brightnessTrackBar.TabIndex = 1;
            this.brightnessTrackBar.TickStyle = System.Windows.Forms.TickStyle.None;
            this.brightnessTrackBar.Value = 50;
            this.brightnessTrackBar.Scroll += new System.EventHandler(this.BrightnessTrackBar_Scroll);

            // 
            // contentPanel
            // 
            this.contentPanel.Dock = System.Windows.Forms.DockStyle.Fill;
            this.contentPanel.Location = new System.Drawing.Point(200, 0);
            this.contentPanel.Name = "contentPanel";
            this.contentPanel.Size = new System.Drawing.Size(880, 640);
            this.contentPanel.TabIndex = 1;

            // 
            // trayMenu
            // 
            this.trayMenu.Items.AddRange(new System.Windows.Forms.ToolStripItem[] {
            this.menuShow,
            this.menuExit});
            this.trayMenu.Name = "trayMenu";
            this.trayMenu.Size = new System.Drawing.Size(104, 48);

            // 
            // menuShow
            // 
            this.menuShow.Name = "menuShow";
            this.menuShow.Size = new System.Drawing.Size(103, 22);
            this.menuShow.Text = "Show";
            this.menuShow.Click += new System.EventHandler(this.MenuShow_Click);

            // 
            // menuExit
            // 
            this.menuExit.Name = "menuExit";
            this.menuExit.Size = new System.Drawing.Size(103, 22);
            this.menuExit.Text = "Exit";
            this.menuExit.Click += new System.EventHandler(this.MenuExit_Click);

            // 
            // trayIcon
            // 
            this.trayIcon.ContextMenuStrip = this.trayMenu;
            this.trayIcon.Text = "Lenovo Legion RGB Controller";
            this.trayIcon.Visible = true;
            this.trayIcon.DoubleClick += new System.EventHandler(this.TrayIcon_DoubleClick);

            // 
            // MainForm
            // 
            this.AutoScaleDimensions = new System.Drawing.SizeF(7F, 15F);
            this.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Font;
            this.ClientSize = new System.Drawing.Size(1080, 640);
            this.Controls.Add(this.contentPanel);
            this.Controls.Add(this.sidebarPanel);
            this.Name = "MainForm";
            this.StartPosition = System.Windows.Forms.FormStartPosition.CenterScreen;
            this.Text = "Legion RGB Controller";
            this.sidebarPanel.ResumeLayout(false);
            this.brandPanel.ResumeLayout(false);
            this.footerPanel.ResumeLayout(false);
            this.footerPanel.PerformLayout();
            ((System.ComponentModel.ISupportInitialize)(this.brightnessTrackBar)).EndInit();
            this.trayMenu.ResumeLayout(false);
            this.ResumeLayout(false);
        }
    }
}
