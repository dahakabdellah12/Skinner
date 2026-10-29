using System;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Threading;

namespace BrawlhallaSkinStudio.App;

/// <summary>
/// Interaction logic for App.xaml
/// </summary>
public partial class App : Application
{
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetForegroundWindow(IntPtr hWnd);

    [DllImport("user32.dll")]
    private static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);

    private const int SW_RESTORE = 9;

    private static readonly string AppDataLogDir = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
        "BrawlhallaSkinStudio", "Logs");

    public static void SafeLog(string filename, string message)
    {
        try
        {
            if (!Directory.Exists(AppDataLogDir))
            {
                Directory.CreateDirectory(AppDataLogDir);
            }
            string logPath = Path.Combine(AppDataLogDir, filename);
            File.AppendAllText(logPath, message);
        }
        catch { }
    }

    public App()
    {
        // Enforce InvariantCulture across all application threads so SVG numeric formatting (decimals as dots) is never broken on European/Arabic/other locales
        System.Globalization.CultureInfo.DefaultThreadCurrentCulture = System.Globalization.CultureInfo.InvariantCulture;
        System.Globalization.CultureInfo.DefaultThreadCurrentUICulture = System.Globalization.CultureInfo.InvariantCulture;
        System.Threading.Thread.CurrentThread.CurrentCulture = System.Globalization.CultureInfo.InvariantCulture;
        System.Threading.Thread.CurrentThread.CurrentUICulture = System.Globalization.CultureInfo.InvariantCulture;

        SafeLog("startup.log", $"[{DateTime.Now:HH:mm:ss.fff}] App constructor PID={Environment.ProcessId}\n");

        DispatcherUnhandledException += (object sender, DispatcherUnhandledExceptionEventArgs e) =>
        {
            SafeLog("app_crash.log", $"[{DateTime.UtcNow}] Dispatcher exception: {e.Exception}\n");
        };

        AppDomain.CurrentDomain.UnhandledException += (object sender, UnhandledExceptionEventArgs e) =>
        {
            SafeLog("app_crash.log", $"[{DateTime.UtcNow}] AppDomain exception: {e.ExceptionObject}\n");
        };

        TaskScheduler.UnobservedTaskException += (object? sender, UnobservedTaskExceptionEventArgs e) =>
        {
            SafeLog("app_crash.log", $"[{DateTime.UtcNow}] Task exception: {e.Exception}\n");
        };

        Exit += (object sender, ExitEventArgs e) =>
        {
            SafeLog("app_exit.log", $"[{DateTime.UtcNow}] Application Exit event with code: {e.ApplicationExitCode}\n");
        };
    }

    protected override void OnStartup(StartupEventArgs e)
    {
        SafeLog("startup.log", $"[{DateTime.Now:HH:mm:ss.fff}] App.OnStartup PID={Environment.ProcessId}\n");

        try
        {
            var currentProcess = Process.GetCurrentProcess();
            var runningInstances = Process.GetProcessesByName(currentProcess.ProcessName)
                                          .Where(p => p.Id != currentProcess.Id)
                                          .ToList();

            SafeLog("startup.log", $"[{DateTime.Now:HH:mm:ss.fff}] Found {runningInstances.Count} other instances of {currentProcess.ProcessName}\n");

            if (runningInstances.Count > 0)
            {
                // Check if any instance actually has a visible main window
                var activeWithWindow = runningInstances.FirstOrDefault(p => p.MainWindowHandle != IntPtr.Zero);
                if (activeWithWindow != null)
                {
                    SafeLog("startup.log", $"[{DateTime.Now:HH:mm:ss.fff}] Activating existing instance PID={activeWithWindow.Id} and shutting down\n");
                    ShowWindow(activeWithWindow.MainWindowHandle, SW_RESTORE);
                    SetForegroundWindow(activeWithWindow.MainWindowHandle);
                    Shutdown(0);
                    return;
                }

                // If orphaned headless instances exist without a window, clean them up
                foreach (var orphan in runningInstances)
                {
                    try { orphan.Kill(); } catch { }
                }
            }
        }
        catch (Exception ex)
        {
            SafeLog("startup.log", $"[{DateTime.Now:HH:mm:ss.fff}] Exception in OnStartup check: {ex}\n");
        }

        base.OnStartup(e);
    }
}
