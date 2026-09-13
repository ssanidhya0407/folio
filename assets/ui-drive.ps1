<#
.SYNOPSIS
  Drive the running folio window and capture screenshots.

.DESCRIPTION
  A smoke-test harness for the GUI: finds the folio window, positions it at a
  fixed 16:9 size, sends clicks and keystrokes, and saves PNGs. Used both to
  check that panels actually work and to produce Store screenshots at the
  1920x1080 the Store expects.

  Coordinates are given in *window-relative* pixels so a shot taken on one
  machine lines up on another.

.EXAMPLE
  ./assets/ui-drive.ps1 -Shot start
  ./assets/ui-drive.ps1 -Click 1180,300 -Shot merge
#>

[CmdletBinding()]
param(
    [string]$Shot,
    [string]$Click,
    [string]$Drag,
    [string]$Type,
    [string]$OutDir = "$PSScriptRoot\..\target\screenshots",
    [int]$Width = 1600,
    [int]$Height = 900,
    [switch]$Position
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms, System.Drawing

if (-not ("Win32" -as [type])) {
    Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win32 {
    // folio is DPI-aware; PowerShell is not. Without matching it, Windows
    // virtualises the coordinates we get back and the captures come out
    // misaligned and the wrong size.
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h,int x,int y,int w,int t,bool r);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f,uint x,uint y,uint d,int e);
    public struct RECT { public int Left, Top, Right, Bottom; }
    public const uint LEFTDOWN = 0x02, LEFTUP = 0x04;
}
"@
}

[Win32]::SetProcessDPIAware() | Out-Null

$proc = Get-Process -Name folio -ErrorAction SilentlyContinue |
        Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $proc) { throw "folio is not running. Start target\release\folio.exe first." }
$hwnd = $proc.MainWindowHandle

if ($Position) {
    [Win32]::MoveWindow($hwnd, 40, 40, $Width, $Height, $true) | Out-Null
    Start-Sleep -Milliseconds 700
}

[Win32]::SetForegroundWindow($hwnd) | Out-Null
Start-Sleep -Milliseconds 300

$rect = New-Object Win32+RECT
[Win32]::GetWindowRect($hwnd, [ref]$rect) | Out-Null

if ($Click) {
    $parts = $Click -split ','
    $x = $rect.Left + [int]$parts[0]
    $y = $rect.Top + [int]$parts[1]
    [Win32]::SetCursorPos($x, $y) | Out-Null
    Start-Sleep -Milliseconds 150
    [Win32]::mouse_event([Win32]::LEFTDOWN, 0, 0, 0, 0)
    Start-Sleep -Milliseconds 60
    [Win32]::mouse_event([Win32]::LEFTUP, 0, 0, 0, 0)
    Start-Sleep -Milliseconds 900
}

if ($Drag) {
    # "x1,y1,x2,y2" — press, move in steps so the page sees pointermove, release.
    $d = $Drag -split ','
    $x1 = $rect.Left + [int]$d[0]; $y1 = $rect.Top + [int]$d[1]
    $x2 = $rect.Left + [int]$d[2]; $y2 = $rect.Top + [int]$d[3]
    [Win32]::SetCursorPos($x1, $y1) | Out-Null
    Start-Sleep -Milliseconds 200
    [Win32]::mouse_event([Win32]::LEFTDOWN, 0, 0, 0, 0)
    for ($i = 1; $i -le 12; $i++) {
        $x = $x1 + [int](($x2 - $x1) * $i / 12)
        $y = $y1 + [int](($y2 - $y1) * $i / 12)
        [Win32]::SetCursorPos($x, $y) | Out-Null
        Start-Sleep -Milliseconds 40
    }
    Start-Sleep -Milliseconds 150
    [Win32]::mouse_event([Win32]::LEFTUP, 0, 0, 0, 0)
    Start-Sleep -Milliseconds 900
}

if ($Type) {
    [System.Windows.Forms.SendKeys]::SendWait($Type)
    Start-Sleep -Milliseconds 900
}

if ($Shot) {
    New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
    [Win32]::GetWindowRect($hwnd, [ref]$rect) | Out-Null
    $w = $rect.Right - $rect.Left
    $h = $rect.Bottom - $rect.Top
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($rect.Left, $rect.Top, 0, 0, (New-Object System.Drawing.Size $w, $h))
    $path = Join-Path $OutDir "$Shot.png"
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bmp.Dispose()
    Write-Output "$path  ($w x $h)"
}
