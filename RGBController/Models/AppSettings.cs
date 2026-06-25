using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace RGBController.Models
{
    public class AppSettings
    {
        [JsonPropertyName("auto_fix_on_startup")]
        public bool AutoFixOnStartup { get; set; }

        [JsonPropertyName("startup_delay_seconds")]
        public uint StartupDelaySeconds { get; set; }

        [JsonPropertyName("fix_on_app_launch")]
        public bool FixOnAppLaunch { get; set; }

        [JsonPropertyName("brightness_level")]
        public float BrightnessLevel { get; set; }

        [JsonPropertyName("ambient_sample_left_fraction")]
        public float AmbientSampleLeftFraction { get; set; }

        [JsonPropertyName("ambient_sample_width_fraction")]
        public float AmbientSampleWidthFraction { get; set; }

        [JsonPropertyName("preset_cycle_shortcut")]
        public string? PresetCycleShortcut { get; set; }

        [JsonPropertyName("preset_cycle_effects")]
        public List<string> PresetCycleEffects { get; set; } = new();

        [JsonPropertyName("preset_tweaks")]
        public Dictionary<string, Dictionary<string, ParameterValue>> PresetTweaks { get; set; } = new();
    }
}
