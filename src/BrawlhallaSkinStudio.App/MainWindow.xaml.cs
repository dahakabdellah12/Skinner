using System;
using System.Diagnostics;
using System.IO;
using System.Net.Http;
using System.Threading.Tasks;
using System.Windows;
using BrawlhallaSkinStudio.App.Bridge;
using BrawlhallaSkinStudio.Core.Engine;
using BrawlhallaSkinStudio.Core.Models;
using BrawlhallaSkinStudio.Core.Services;
using Microsoft.Web.WebView2.Core;

namespace BrawlhallaSkinStudio.App;

public partial class MainWindow : Window
{
    private WebViewBridge? _bridge;
    private GameInstallation _installation = new();
    private readonly CatalogService _catalog = new();
    private BackupManager? _backupManager;
    private ModificationEngine? _modEngine;
    private PresetManager? _presetManager;

    public MainWindow()
    {
        InitializeComponent();
        Loaded += OnWindowLoaded;
        Closing += (s, e) => App.SafeLog("app_exit.log", $"[{DateTime.UtcNow}] MainWindow Closing: Cancel={e.Cancel}\n");
        Closed += (s, e) =>
        {
            App.SafeLog("app_exit.log", $"[{DateTime.UtcNow}] MainWindow Closed\n");
            try
            {
                MainWebView?.Dispose();
            }
            catch { }
        };
    }

    protected override void OnSourceInitialized(EventArgs e)
    {
        base.OnSourceInitialized(e);

        if (Height > SystemParameters.WorkArea.Height)
        {
            Height = Math.Max(MinHeight, SystemParameters.WorkArea.Height - 24);
            Top = 12;
        }
        if (Width > SystemParameters.WorkArea.Width)
        {
            Width = Math.Max(MinWidth, SystemParameters.WorkArea.Width - 24);
            Left = 12;
        }
    }

    protected override void OnStateChanged(EventArgs e)
    {
        base.OnStateChanged(e);
        NotifyWindowState();
    }

    private double _restoreLeft;
    private double _restoreTop;
    private double _restoreWidth;
    private double _restoreHeight;
    private bool _isCustomMaximized;

    private bool ToggleMaximize()
    {
        if (!_isCustomMaximized)
        {
            _restoreLeft = Left;
            _restoreTop = Top;
            _restoreWidth = Width;
            _restoreHeight = Height;

            var wa = SystemParameters.WorkArea;
            Left = wa.Left;
            Top = wa.Top;
            Width = wa.Width;
            Height = wa.Height;
            _isCustomMaximized = true;
        }
        else
        {
            Left = _restoreLeft;
            Top = _restoreTop;
            Width = _restoreWidth;
            Height = _restoreHeight;
            _isCustomMaximized = false;
        }

        NotifyWindowState();
        return _isCustomMaximized;
    }

    private void NotifyWindowState()
    {
        try
        {
            bool isMaximized = _isCustomMaximized || WindowState == WindowState.Maximized;
            MainWebView?.CoreWebView2?.ExecuteScriptAsync($"window.__setWindowState && window.__setWindowState({(isMaximized ? "true" : "false")});");
        }
        catch { }
    }

    private async void OnWindowLoaded(object sender, RoutedEventArgs e)
    {
        void Log(string msg) => App.SafeLog("startup.log", $"[{DateTime.Now:HH:mm:ss.fff}] {msg}\n");

        try
        {
            Log("1. Window loaded");
            // 1. Initialize C# Backend Services in background
            await Task.Run(() =>
            {
                Log("2. Task.Run starting detection");
                _installation = GameDetector.DetectGameInstallation();
                Log($"3. Game detected: Valid={_installation.IsValid}, Path={_installation.GamePath}");
                if (_installation.IsValid)
                {
                    _backupManager = new BackupManager(_installation.GamePath);
                    _catalog.LoadFromInstallation(_installation);
                    _modEngine = new ModificationEngine(_installation.GamePath, _backupManager, _catalog);
                    _presetManager = new PresetManager(Path.Combine(_installation.GamePath, ".studio_presets"));
                    Log($"4. Catalog loaded: {_catalog.Legends.Count} legends, {_catalog.Costumes.Count} costumes");
                }
            });

            // Fallbacks for empty/uninitialized instances to prevent null reference
            string fallbackDir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "BrawlhallaSkinStudio");
            _backupManager ??= new BackupManager(fallbackDir);
            _modEngine ??= new ModificationEngine(fallbackDir, _backupManager, _catalog);
            _presetManager ??= new PresetManager(Path.Combine(fallbackDir, ".studio_presets"));

            Log("5. Initializing WebView2 environment");
            // 2. Initialize WebView2 Environment with resilient retry and fallback
            CoreWebView2Environment env;
            string primaryUserDataFolder = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "BrawlhallaSkinStudio", "WebView2Data");

            try
            {
                Directory.CreateDirectory(primaryUserDataFolder);
                env = await CoreWebView2Environment.CreateAsync(userDataFolder: primaryUserDataFolder);
                Log("6. CoreWebView2Environment created with primary folder");
                await MainWebView.EnsureCoreWebView2Async(env);
                Log("7. EnsureCoreWebView2Async complete");
            }
            catch (Exception ex)
            {
                Log($"Primary WebView2 initialization failed ({ex.Message}, HR=0x{ex.HResult:X8}). Attempting isolated session fallback...");
                string fallbackFolder = Path.Combine(Path.GetTempPath(), "BrawlhallaSkinStudio", $"WebView2_{Environment.ProcessId}_{Guid.NewGuid():N}");
                Directory.CreateDirectory(fallbackFolder);
                env = await CoreWebView2Environment.CreateAsync(userDataFolder: fallbackFolder);
                Log($"6b. CoreWebView2Environment created with fallback folder: {fallbackFolder}");
                await MainWebView.EnsureCoreWebView2Async(env);
                Log("7b. EnsureCoreWebView2Async complete with fallback");
            }

            var core = MainWebView.CoreWebView2;

            // 3. Configure Security and Origin
            // JavaScript alert/confirm/prompt dialogs are never used by the UI; disabling them
            // prevents hostile page content from blocking the WebView thread.
            core.Settings.AreDefaultScriptDialogsEnabled = false;
            core.Settings.IsStatusBarEnabled = false;

            // Restrict Navigation to authorized local or dev origins
            core.NavigationStarting += (s, args) =>
            {
                bool isAllowed = false;
                if (Uri.TryCreate(args.Uri, UriKind.Absolute, out var uri))
                {
                    isAllowed = (uri.Host.Equals("localhost", StringComparison.OrdinalIgnoreCase) && uri.Port == 5173) ||
                                uri.Host.Equals("appassets.local", StringComparison.OrdinalIgnoreCase);
                }

                if (!isAllowed)
                {
                    args.Cancel = true;
                    // Open external links in the default browser, but only over HTTP(S).
                    if (IsSafeExternalUri(args.Uri))
                    {
                        try
                        {
                            Process.Start(new ProcessStartInfo { FileName = args.Uri, UseShellExecute = true });
                        }
                        catch { }
                    }
                }
            };

            // Intercept new window requests (e.g. target="_blank")
            core.NewWindowRequested += (s, args) =>
            {
                args.Handled = true;
                if (IsSafeExternalUri(args.Uri))
                {
                    try
                    {
                        Process.Start(new ProcessStartInfo { FileName = args.Uri, UseShellExecute = true });
                    }
                    catch { }
                }
            };

            // 4. Resolve Frontend Folder & Map Virtual Host
            string frontendDistPath = ResolveFrontendDistPath();
            Log($"8. Frontend dist path: {frontendDistPath} (exists={Directory.Exists(frontendDistPath)})");
            if (Directory.Exists(frontendDistPath))
            {
                core.SetVirtualHostNameToFolderMapping(
                    "appassets.local",
                    frontendDistPath,
                    CoreWebView2HostResourceAccessKind.Allow);
            }

            // 4b. Global script to forward any runtime uncaught exceptions directly to backend bridge
            await core.AddScriptToExecuteOnDocumentCreatedAsync(@"
                window.addEventListener('error', function(e) {
                    try {
                        if (window.chrome && window.chrome.webview) {
                            window.chrome.webview.postMessage(JSON.stringify({
                                id: 'err_' + Date.now(),
                                action: 'logClientError',
                                payload: {
                                    message: e.message || 'Script Error',
                                    stack: e.error ? e.error.stack : (e.filename + ':' + e.lineno + ':' + e.colno)
                                }
                            }));
                        }
                    } catch {}
                });
                window.addEventListener('unhandledrejection', function(e) {
                    try {
                        if (window.chrome && window.chrome.webview) {
                            window.chrome.webview.postMessage(JSON.stringify({
                                id: 'rej_' + Date.now(),
                                action: 'logClientError',
                                payload: {
                                    message: 'Unhandled Promise Rejection: ' + (e.reason ? (e.reason.message || String(e.reason)) : 'unknown'),
                                    stack: e.reason ? (e.reason.stack || '') : ''
                                }
                            }));
                        }
                    } catch {}
                });
            ");

            // 5. Attach C# WebView Bridge
            _bridge = new WebViewBridge(
                core,
                _installation,
                _catalog,
                _backupManager,
                _modEngine,
                _presetManager,
                onMinimize: () => Dispatcher.Invoke(() => WindowState = WindowState.Minimized),
                onMaximize: () => Dispatcher.Invoke(() => ToggleMaximize()),
                onClose: () => Dispatcher.Invoke(Close),
                onDrag: () => Dispatcher.Invoke(() =>
                {
                    try
                    {
                        if (System.Windows.Input.Mouse.LeftButton == System.Windows.Input.MouseButtonState.Pressed)
                        {
                            if (_isCustomMaximized)
                            {
                                ToggleMaximize();
                            }
                            DragMove();
                        }
                    }
                    catch { }
                }),
                isWindowMaximized: () => Dispatcher.Invoke(() => _isCustomMaximized || WindowState == WindowState.Maximized));
            Log("9. Bridge attached");

            // 6. Check if Vite Dev Server is running on http://localhost:5173
            bool isDevServerRunning = await CheckDevServerRunningAsync("http://localhost:5173");
            Log($"10. Dev server running: {isDevServerRunning}");

            if (isDevServerRunning)
            {
                core.Navigate("http://localhost:5173");
            }
            else
            {
                core.Navigate("https://appassets.local/index.html");
            }
            Log("11. Navigate called");
        }
        catch (Exception ex)
        {
            Log($"ERROR in OnWindowLoaded: {ex}");
            MessageBox.Show(
                $"Failed to initialize WebView2: {ex.Message}\n\nPlease ensure the Microsoft Edge WebView2 Runtime is installed on Windows.",
                "BrawlCraft - Initialization Error",
                MessageBoxButton.OK,
                MessageBoxImage.Error);
        }
    }

    private static string ResolveFrontendDistPath()
    {
        string baseDir = AppDomain.CurrentDomain.BaseDirectory;
        string[] candidates =
        [
            Path.Combine(baseDir, "frontend", "dist"),
            Path.Combine(baseDir, "dist"),
            Path.GetFullPath(Path.Combine(baseDir, "..", "..", "..", "..", "frontend", "dist")),
            Path.GetFullPath(Path.Combine(baseDir, "..", "..", "..", "frontend", "dist")),
            Path.GetFullPath(Path.Combine(baseDir, "..", "frontend", "dist"))
        ];

        foreach (var c in candidates)
        {
            if (Directory.Exists(c) && File.Exists(Path.Combine(c, "index.html")))
            {
                return c;
            }
        }

        // Default to candidate 0
        return candidates[0];
    }

    private static bool IsSafeExternalUri(string? uriString)
    {
        return Uri.TryCreate(uriString, UriKind.Absolute, out var uri)
               && (uri.Scheme == Uri.UriSchemeHttp || uri.Scheme == Uri.UriSchemeHttps);
    }

    private static async Task<bool> CheckDevServerRunningAsync(string url)
    {
        try
        {
            using var client = new HttpClient { Timeout = TimeSpan.FromMilliseconds(400) };
            var response = await client.GetAsync(url);
            return response.IsSuccessStatusCode;
        }
        catch
        {
            return false;
        }
    }
}
