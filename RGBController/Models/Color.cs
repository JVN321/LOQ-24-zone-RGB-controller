using System.Runtime.InteropServices;

namespace RGBController.Models
{
    [StructLayout(LayoutKind.Sequential, Pack = 1)]
    public struct Color
    {
        public byte r;
        public byte g;
        public byte b;

        public Color(byte r, byte g, byte b)
        {
            this.r = r;
            this.g = g;
            this.b = b;
        }

        public static Color Black => new Color(0, 0, 0);
        public static Color White => new Color(255, 255, 255);
    }
}
