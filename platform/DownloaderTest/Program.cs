using System;
using System.Threading.Tasks;
using WickDownloaderSharp;

namespace DownloaderTest
{
    class Program
    {
        public static async Task Main(string[] args)
        {
            var rt = await Runtime.Initialize();
            var names = rt.GetPakNames();
            var first_pak = names[0];
            var pak = await rt.GetPakService(first_pak, "6c51aba88ca1240a0d14eb94701f6c41fd7799b102e9060d1e6c316993196fdf");
            var first_file = pak.GetFileNames()[0];
            var first_hash = pak.GetFileHash(first_file);
            Console.WriteLine(first_hash);
            Console.ReadLine();
            //rt.Dispose();
        }
    }
}
