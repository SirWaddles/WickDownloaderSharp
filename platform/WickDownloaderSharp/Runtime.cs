using System;
using System.Runtime.InteropServices;
using System.Threading.Tasks;
using System.Collections.Generic;
using System.Text;

internal class RuntimeBindings
{
    public delegate void NotifyDelegate(int a);

    [DllImport("wick_downloader.dll")]
    internal static extern RuntimeHandle initialize();

    [DllImport("wick_downloader.dll")]
    internal static extern void destroy(IntPtr handle);

    [DllImport("wick_downloader.dll")]
    internal static extern void notify_me(RuntimeHandle handle, NotifyDelegate cb);
}

internal class RuntimeHandle : SafeHandle
{
    public RuntimeHandle() : base(IntPtr.Zero, true) { }
    public override bool IsInvalid
    {
        get { return handle == IntPtr.Zero; }
    }

    protected override bool ReleaseHandle()
    {
        System.Console.WriteLine("RuntimeHandle Dispose");
        RuntimeBindings.destroy(handle);
        return true;
    }
}

namespace WickDownloaderSharp
{
    public class Runtime : IDisposable
    {
        private RuntimeHandle handle;
        public Runtime()
        {
            handle = RuntimeBindings.initialize();
        }

        public Task<int> notify_test()
        {
            var taskPlace = new TaskCompletionSource<int>();
            var callbackHandle = default(GCHandle);
            RuntimeBindings.NotifyDelegate nativeCallback = a =>
            {
                try
                {
                    taskPlace.SetResult(a);
                } finally
                {
                    if (callbackHandle.IsAllocated)
                    {
                        callbackHandle.Free();
                    }
                }
            };
            callbackHandle = GCHandle.Alloc(nativeCallback);
            System.Console.WriteLine("Starting Notify");
            RuntimeBindings.notify_me(handle, nativeCallback);
            System.Console.WriteLine("Notify Returned");
            return taskPlace.Task;
        }

        public void Dispose()
        {
            System.Console.WriteLine("Runtime Dipose");
            Dispose(true);
            GC.SuppressFinalize(this);
        }

        protected virtual void Dispose(bool disposing)
        {
            if (handle != null && !handle.IsInvalid)
            {
                handle.Dispose();
            }
        }
    }
}
