using System;
using System.IO;

namespace RGBController
{
    public static class SettingsManager
    {
        private static readonly string AppDataFolder = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
            "LightingControl"
        );
        private static readonly string ActivePresetFile = Path.Combine(AppDataFolder, "active_preset.txt");

        static SettingsManager()
        {
            try
            {
                if (!Directory.Exists(AppDataFolder))
                {
                    Directory.CreateDirectory(AppDataFolder);
                }
            }
            catch
            {
                // Ignore folder creation errors
            }
        }

        public static string GetActivePreset()
        {
            try
            {
                if (File.Exists(ActivePresetFile))
                {
                    return File.ReadAllText(ActivePresetFile).Trim();
                }
            }
            catch
            {
                // Ignore read errors
            }
            return "StaticColor"; // Default fallback preset
        }

        public static void SaveActivePreset(string presetName)
        {
            try
            {
                File.WriteAllText(ActivePresetFile, presetName);
            }
            catch
            {
                // Ignore write errors
            }
        }
    }
}
