using System;
using System.Runtime.InteropServices;
using System.Threading.Tasks;
using System.Collections.Generic;
using System.Text;

namespace WickDownloader
{
    public class WickException : Exception
    {
        public WickException(uint code, string message) : base(GetMessage(code, message))
        {
            
        }

        private static string GetMessage(uint code, string message)
        {
            return GetMessageCode(code) + ": " + message;
        }

        private static string GetMessageCode(uint code)
        {
            switch (code)
            {
                case 1:
                    return "HTTP Error";
                case 2:
                    return "Header String Error";
                case 3:
                    return "HTTP Request Error";
                case 4:
                    return "UTF-8 Error";
                case 5:
                    return "JSON Error";
                case 6:
                    return "Reader Error";
                case 7:
                    return "Async Error";
                case 8:
                    return "JWP Error";
                case 9:
                    return "Key Error";
                case 10:
                    return "Decrypt Error";
                case 11:
                    return "Hex Error";
                default:
                    return "Unknown Error";
            }
        }
    }

    public class WickInitException : WickException
    {
        private Runtime rt;
        internal WickInitException(uint code, string message, Runtime handle) : base(code, message)
        {
            rt = handle;
        }
    }

    [StructLayout(LayoutKind.Sequential)]
    internal class FileDataReturn
    {
        public IntPtr hash;
        public uint error;
    }

    internal class RuntimeBindings
    {
        public delegate void InitializeDelegate(IntPtr a, uint err);
        public delegate void PakRetrieveDelegate(IntPtr pakService, uint err);
        public delegate void DataRetrieveDelegate(IntPtr data, uint length, uint err);
        public delegate void VoidReturnDelegate(uint err);

        [DllImport("wick_downloader.dll")]
        internal static extern void initialize(InitializeDelegate a);

        [DllImport("wick_downloader.dll")]
        internal static extern void initialize_with_manifest(string app_manifest, string chunk_manifest, InitializeDelegate cb);

        [DllImport("wick_downloader.dll")]
        internal static extern void destroy(IntPtr handle);

        [DllImport("wick_downloader.dll")]
        internal static extern VecStringHandle get_pak_names(RuntimeHandle handle);

        [DllImport("wick_downloader.dll")]
        internal static extern void get_pak(RuntimeHandle handle, string file, PakRetrieveDelegate cb);

        [DllImport("wick_downloader.dll")]
        internal static extern void download_file(RuntimeHandle handle, string pak, string file, VoidReturnDelegate cb);

        [DllImport("wick_downloader.dll")]
        internal static extern StringHandle get_pak_mount(PakHandle handle);

        [DllImport("wick_downloader.dll")]
        internal static extern VecStringHandle get_file_names(PakHandle handle);

        [DllImport("wick_downloader.dll")]
        internal static extern void get_file_data(RuntimeHandle handle, PakHandle phandle, string file, DataRetrieveDelegate cb);

        [DllImport("wick_downloader.dll")]
        internal static extern StringHandle vec_string_get_next(VecStringHandle handle);

        [DllImport("wick_downloader.dll")]
        internal static extern StringHandle get_last_error();

        public static string GetLastError()
        {
            var handle = get_last_error();
            var message = handle.AsString();
            handle.Dispose();
            return message;
        }

        [DllImport("wick_downloader.dll")]
        internal static extern void free_pak(IntPtr handle);

        [DllImport("wick_downloader.dll")]
        internal static extern void free_encrypted_pak(IntPtr handle);

        [DllImport("wick_downloader.dll")]
        internal static extern void free_vec_string(IntPtr handle);

        [DllImport("wick_downloader.dll")]
        internal static extern void free_string(IntPtr handle);
    }

    internal class RuntimeHandle : SafeHandle
    {
        public RuntimeHandle() : base(IntPtr.Zero, true) { }
        public RuntimeHandle(IntPtr ptr) : base (IntPtr.Zero, true)
        {
            handle = ptr;
        }
        public override bool IsInvalid
        {
            get { return handle == IntPtr.Zero; }
        }

        protected override bool ReleaseHandle()
        {
            RuntimeBindings.destroy(handle);
            return true;
        }
    }

    internal class PakHandle : SafeHandle
    {
        public PakHandle(IntPtr ptr) : base (IntPtr.Zero, true)
        {
            handle = ptr;
        }
        public override bool IsInvalid
        {
            get { return handle == IntPtr.Zero; }
        }
        protected override bool ReleaseHandle()
        {
            RuntimeBindings.free_pak(handle);
            return true;
        }
    }

    internal class VecStringHandle : SafeHandle
    {
        public VecStringHandle() : base(IntPtr.Zero, true) { }
        public override bool IsInvalid
        {
            get { return handle == IntPtr.Zero; }
        }

        protected override bool ReleaseHandle()
        {
            RuntimeBindings.free_vec_string(handle);
            return true;
        }
    }

    internal class StringHandle : SafeHandle
    {
        public StringHandle() : base(IntPtr.Zero, true) { }
        public StringHandle(IntPtr ptr) : base (IntPtr.Zero, true)
        {
            handle = ptr;
        }

        public override bool IsInvalid
        {
            get { return handle == IntPtr.Zero; }
        }

        protected override bool ReleaseHandle()
        {
            RuntimeBindings.free_string(handle);
            return true;
        }

        public string AsString()
        {
            int len = 0;
            while (Marshal.ReadByte(handle, len) != 0) { ++len; }
            byte[] buffer = new byte[len];
            Marshal.Copy(handle, buffer, 0, buffer.Length);
            return Encoding.UTF8.GetString(buffer);
        }
    }

    public class PakService : IDisposable
    {
        internal PakHandle handle;

        internal PakService(PakHandle pakhandle)
        {
            handle = pakhandle;
        }

        public List<string> GetFileNames()
        {
            VecStringHandle namehandle = RuntimeBindings.get_file_names(handle);
            StringHandle testHandle = RuntimeBindings.vec_string_get_next(namehandle);
            var names = new List<string>();
            while (!testHandle.IsInvalid)
            {
                names.Add(testHandle.AsString());
                testHandle.Dispose();
                testHandle = RuntimeBindings.vec_string_get_next(namehandle);
            }

            namehandle.Dispose();
            return names;
        }

        public string GetMountPath()
        {
            var pathHandle = RuntimeBindings.get_pak_mount(handle);
            var path = pathHandle.AsString();
            pathHandle.Dispose();
            return path;
        }

        public void Dispose()
        {
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
    public class Runtime : IDisposable
    {
        private RuntimeHandle handle;
        private Runtime(RuntimeHandle runtime)
        {
            handle = runtime;
        }

        public Runtime(string app_manifest, string chunk_manifest)
        {
            void cb(IntPtr service, uint err)
            {
                if (err != 0)
                {
                    throw new WickException(err, RuntimeBindings.GetLastError());
                }
                handle = new RuntimeHandle(service);
            }
            RuntimeBindings.initialize_with_manifest(app_manifest, chunk_manifest, cb);
            
        }

        public static Task<Runtime> Initialize()
        {
            var taskPlace = new TaskCompletionSource<Runtime>(TaskCreationOptions.RunContinuationsAsynchronously);
            var callbackHandle = default(GCHandle);
            RuntimeBindings.InitializeDelegate nativeCallback = (a, err) =>
            {
                try
                {
                    var runtime = new Runtime(new RuntimeHandle(a));
                    if (err != 0)
                    {
                        taskPlace.SetException(new WickInitException(err, RuntimeBindings.GetLastError(), runtime));
                        return;
                    }
                    taskPlace.SetResult(runtime);
                }
                finally
                {
                    if (callbackHandle.IsAllocated)
                    {
                        callbackHandle.Free();
                    }
                }
            };
            callbackHandle = GCHandle.Alloc(nativeCallback);
            RuntimeBindings.initialize(nativeCallback);
            return taskPlace.Task;
        }

        public Task<PakService> GetPak(string file)
        {
            var taskPlace = new TaskCompletionSource<PakService>(TaskCreationOptions.RunContinuationsAsynchronously);
            var callbackHandle = default(GCHandle);
            RuntimeBindings.PakRetrieveDelegate nativeCallback = (a, err) =>
            {
                try
                {
                    if (err != 0)
                    {
                        taskPlace.SetException(new WickException(err, RuntimeBindings.GetLastError()));
                        return;
                    }
                    taskPlace.SetResult(new PakService(new PakHandle(a)));
                }
                finally
                {
                    if (callbackHandle.IsAllocated)
                    {
                        callbackHandle.Free();
                    }
                }
            };
            callbackHandle = GCHandle.Alloc(nativeCallback);
            RuntimeBindings.get_pak(handle, file, nativeCallback);
            return taskPlace.Task;
        }

        public Task<bool> DownloadFile(string pak, string file)
        {
            var taskPlace = new TaskCompletionSource<bool>(TaskCreationOptions.RunContinuationsAsynchronously);
            var callbackHandle = default(GCHandle);
            RuntimeBindings.VoidReturnDelegate nativeCallback = (err) =>
            {
                try
                {
                    if (err != 0)
                    {
                        taskPlace.SetException(new WickException(err, RuntimeBindings.GetLastError()));
                        return;
                    }
                    taskPlace.SetResult(true);
                }
                finally
                {
                    if (callbackHandle.IsAllocated)
                    {
                        callbackHandle.Free();
                    }
                }
            };
            callbackHandle = GCHandle.Alloc(nativeCallback);
            RuntimeBindings.download_file(handle, pak, file, nativeCallback);
            return taskPlace.Task;
        }

        public Task<byte[]> GetPakData(PakService pak, string file)
        {
            var taskPlace = new TaskCompletionSource<byte[]>(TaskCreationOptions.RunContinuationsAsynchronously);
            var callbackHandle = default(GCHandle);
            RuntimeBindings.DataRetrieveDelegate nativeCallback = (data, length, err) =>
            {
                try
                {
                    if (err != 0)
                    {
                        taskPlace.SetException(new WickException(err, RuntimeBindings.GetLastError()));
                        return;
                    }
                    byte[] mdata = new byte[length];
                    Marshal.Copy(data, mdata, 0, Convert.ToInt32(length));
                    taskPlace.SetResult(mdata);
                }
                finally
                {
                    if (callbackHandle.IsAllocated)
                    {
                        callbackHandle.Free();
                    }
                }
            };
            callbackHandle = GCHandle.Alloc(nativeCallback);
            RuntimeBindings.get_file_data(handle, pak.handle, file, nativeCallback);
            return taskPlace.Task;
        }

        public List<string> GetPakNames()
        {
            VecStringHandle namehandle = RuntimeBindings.get_pak_names(handle);
            StringHandle testHandle = RuntimeBindings.vec_string_get_next(namehandle);
            var names = new List<string>();
            while (!testHandle.IsInvalid)
            {
                names.Add(testHandle.AsString());
                testHandle.Dispose();
                testHandle = RuntimeBindings.vec_string_get_next(namehandle);
            }

            namehandle.Dispose();
            return names;
        }

        public void Dispose()
        {
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
