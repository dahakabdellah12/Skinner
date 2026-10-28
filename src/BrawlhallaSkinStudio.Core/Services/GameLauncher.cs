using System;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;

namespace BrawlhallaSkinStudio.Core.Services;

public static class GameLauncher
{
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetForegroundWindow(IntPtr hWnd);

    [DllImport("user32.dll")]
    private static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);

    private const int SW_RESTORE = 9;

    public static bool IsGameRunning()
    {
        return Process.GetProcessesByName("Brawlhalla").Length > 0;
    }

    public static bool BringGameToFront()
    {
        try
        {
            var process = Process.GetProcessesByName("Brawlhalla")
                .FirstOrDefault(p => p.MainWindowHandle != IntPtr.Zero);
            if (process != null)
            {
                ShowWindow(process.MainWindowHandle, SW_RESTORE);
                SetForegroundWindow(process.MainWindowHandle);
                return true;
            }
        }
        catch { }
        return false;
    }

    public static bool LaunchGame(string gameExePath, bool preferSteam = true)
    {
        if (IsGameRunning())
        {
            if (BringGameToFront())
                return true;
        }

        try
        {
            if (preferSteam)
            {
                // Launch via Steam URL protocol
                Process.Start(new ProcessStartInfo
                {
                    FileName = "steam://rungameid/291550",
                    UseShellExecute = true
                });
                return true;
            }
            else
            {
                if (File.Exists(gameExePath))
                {
                    Process.Start(new ProcessStartInfo
                    {
                        FileName = gameExePath,
                        WorkingDirectory = Path.GetDirectoryName(gameExePath),
                        UseShellExecute = true
                    });
                    return true;
                }
            }
        }
        catch { }

        return false;
    }

    public static bool RestartGame(string gameExePath, bool preferSteam = true)
    {
        try
        {
            var processes = Process.GetProcessesByName("Brawlhalla");
            foreach (var p in processes)
            {
                try
                {
                    p.CloseMainWindow();
                    if (!p.WaitForExit(1500))
                    {
                        p.Kill();
                        p.WaitForExit(1000);
                    }
                }
                catch { }
            }

            System.Threading.Thread.Sleep(500);

            if (preferSteam)
            {
                Process.Start(new ProcessStartInfo
                {
                    FileName = "steam://rungameid/291550",
                    UseShellExecute = true
                });
                return true;
            }
            else if (File.Exists(gameExePath))
            {
                Process.Start(new ProcessStartInfo
                {
                    FileName = gameExePath,
                    WorkingDirectory = Path.GetDirectoryName(gameExePath),
                    UseShellExecute = true
                });
                return true;
            }
        }
        catch { }

        return false;
    }

    public static string[] GetVerificationChecklist(string legendName, string costumeName, string weapon1, string weapon2)
    {
        bool isRunning = IsGameRunning();
        string step1 = isRunning
            ? "1. Brawlhalla is currently running! Switch to another Legend and back in Character Select (or re-enter Training Mode) to load the new appearance without restarting the game."
            : "1. Ensure Brawlhalla is launched and loaded to the Main Menu.";

        return
        [
            step1,
            $"2. Navigate to 'Offline Play' -> 'Training Mode' or 'Custom Game Room'.",
            $"3. Select Legend: '{legendName}'.",
            $"4. In the skin selector, select Costume: '{costumeName}'.",
            $"5. In the weapon loadout, inspect Primary Weapon: '{weapon1}'.",
            $"6. Inspect Secondary Weapon: '{weapon2}'.",
            $"7. Enter the match and perform light attacks, heavy signatures, and idle animations.",
            $"8. Confirm visual fidelity, symmetry, and color accuracy in gameplay.",
            $"9. If any visual artifact is observed, use 'Restore Original' in Mod Manager to restore pristine state."
        ];
    }
}
