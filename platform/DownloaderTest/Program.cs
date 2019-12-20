using System;
using System.Threading.Tasks;
using WickDownloaderSharp;

namespace DownloaderTest
{
    class Program
    {
        public static async Task Main(string[] args)
        {
            var rt = new Runtime();
            var task = await rt.notify_test();
            Console.WriteLine("Test: {0}", task);
            Console.ReadLine();
            rt.Dispose();
        }
    }
}
