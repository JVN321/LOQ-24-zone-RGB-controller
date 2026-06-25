using System;
using System.Collections.Generic;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace RGBController.Models
{
    public class PresetMetadata
    {
        [JsonPropertyName("name")]
        public string Name { get; set; } = string.Empty;

        [JsonPropertyName("display_name")]
        public string DisplayName { get; set; } = string.Empty;

        [JsonPropertyName("description")]
        public string Description { get; set; } = string.Empty;

        [JsonPropertyName("parameters")]
        public List<ParameterConfig> Parameters { get; set; } = new();
    }

    public class ParameterConfig
    {
        [JsonPropertyName("name")]
        public string Name { get; set; } = string.Empty;

        [JsonPropertyName("label")]
        public string Label { get; set; } = string.Empty;

        [JsonPropertyName("param_type")]
        public ParameterTypeConfig ParamType { get; set; } = new();

        [JsonPropertyName("min")]
        public float Min { get; set; }

        [JsonPropertyName("max")]
        public float Max { get; set; }

        [JsonPropertyName("default")]
        public float Default { get; set; }

        [JsonPropertyName("step")]
        public float Step { get; set; }
    }

    public class ParameterTypeConfig
    {
        [JsonPropertyName("type")]
        public string Type { get; set; } = string.Empty; // "Float" or "Color"

        [JsonPropertyName("r")]
        public byte? R { get; set; }

        [JsonPropertyName("g")]
        public byte? G { get; set; }

        [JsonPropertyName("b")]
        public byte? B { get; set; }
    }

    public class ParameterValue
    {
        [JsonPropertyName("type")]
        public string Type { get; set; } = string.Empty; // "Float" or "Color"

        [JsonPropertyName("value")]
        public object? Value { get; set; } // Float value or Color JSON object

        [JsonIgnore]
        public float FloatValue
        {
            get
            {
                if (Value is JsonElement element && element.ValueKind == JsonValueKind.Number)
                {
                    return element.GetSingle();
                }
                if (Value is float f) return f;
                if (Value is double d) return (float)d;
                if (Value is int i) return i;
                return 0f;
            }
        }

        [JsonIgnore]
        public Color ColorValue
        {
            get
            {
                if (Value is JsonElement element && element.ValueKind == JsonValueKind.Object)
                {
                    byte r = element.TryGetProperty("r", out var rp) ? rp.GetByte() : (byte)0;
                    byte g = element.TryGetProperty("g", out var gp) ? gp.GetByte() : (byte)0;
                    byte b = element.TryGetProperty("b", out var bp) ? bp.GetByte() : (byte)0;
                    return new Color(r, g, b);
                }
                return Color.Black;
            }
        }

        public static ParameterValue CreateFloat(float val)
        {
            return new ParameterValue { Type = "Float", Value = val };
        }

        public static ParameterValue CreateColor(byte r, byte g, byte b)
        {
            // Must serialize as object with r, g, b fields matching Rust Color
            return new ParameterValue 
            { 
                Type = "Color", 
                Value = new { r, g, b } 
            };
        }
    }
}
