param(
    [string]$NameHint = "Pocket Synth",
    [int]$Note = 60,
    [int]$Velocity = 100,
    [int]$DurationMs = 500,
    [ValidateSet("note", "scale", "twinkle")]
    [string]$Pattern = "note",
    [switch]$List
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public static class WinMMMidiSmoke {
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Ansi)]
    public struct MIDIINCAPS {
        public ushort wMid;
        public ushort wPid;
        public uint vDriverVersion;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)]
        public string szPname;
        public uint dwSupport;
    }

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Ansi)]
    public struct MIDIOUTCAPS {
        public ushort wMid;
        public ushort wPid;
        public uint vDriverVersion;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)]
        public string szPname;
        public ushort wTechnology;
        public ushort wVoices;
        public ushort wNotes;
        public ushort wChannelMask;
        public uint dwSupport;
    }

    [DllImport("winmm.dll")]
    public static extern uint midiInGetNumDevs();

    [DllImport("winmm.dll", EntryPoint = "midiInGetDevCapsA", CharSet = CharSet.Ansi)]
    public static extern uint midiInGetDevCaps(UIntPtr uDeviceID, out MIDIINCAPS caps, uint cbMidiInCaps);

    [DllImport("winmm.dll")]
    public static extern uint midiOutGetNumDevs();

    [DllImport("winmm.dll", EntryPoint = "midiOutGetDevCapsA", CharSet = CharSet.Ansi)]
    public static extern uint midiOutGetDevCaps(UIntPtr uDeviceID, out MIDIOUTCAPS caps, uint cbMidiOutCaps);

    [DllImport("winmm.dll")]
    public static extern uint midiOutOpen(out IntPtr handle, uint deviceID, IntPtr callback, IntPtr instance, uint flags);

    [DllImport("winmm.dll")]
    public static extern uint midiOutShortMsg(IntPtr handle, uint message);

    [DllImport("winmm.dll")]
    public static extern uint midiOutClose(IntPtr handle);
}
"@

function New-MidiMessage([int]$Status, [int]$Data1, [int]$Data2) {
    return [uint32]($Status -bor ($Data1 -shl 8) -bor ($Data2 -shl 16))
}

function Send-MidiMessage([IntPtr]$Handle, [uint32]$Message) {
    $rc = [WinMMMidiSmoke]::midiOutShortMsg($Handle, $Message)
    if ($rc -ne 0) {
        throw "midiOutShortMsg failed: $rc"
    }
}

function Send-Note([IntPtr]$Handle, [int]$MidiNote, [int]$MidiVelocity, [int]$LengthMs) {
    Send-MidiMessage $Handle (New-MidiMessage 0x90 $MidiNote $MidiVelocity)
    Start-Sleep -Milliseconds $LengthMs
    Send-MidiMessage $Handle (New-MidiMessage 0x80 $MidiNote 0)
}

function Get-MidiOutputs {
    $count = [WinMMMidiSmoke]::midiOutGetNumDevs()
    $size = [Runtime.InteropServices.Marshal]::SizeOf([type][WinMMMidiSmoke+MIDIOUTCAPS])
    for ($i = 0; $i -lt $count; $i++) {
        $caps = New-Object WinMMMidiSmoke+MIDIOUTCAPS
        $rc = [WinMMMidiSmoke]::midiOutGetDevCaps([UIntPtr]::new([uint64]$i), [ref]$caps, [uint32]$size)
        [pscustomobject]@{
            Direction = "out"
            Id = $i
            Status = $rc
            Name = $caps.szPname
        }
    }
}

function Get-MidiInputs {
    $count = [WinMMMidiSmoke]::midiInGetNumDevs()
    $size = [Runtime.InteropServices.Marshal]::SizeOf([type][WinMMMidiSmoke+MIDIINCAPS])
    for ($i = 0; $i -lt $count; $i++) {
        $caps = New-Object WinMMMidiSmoke+MIDIINCAPS
        $rc = [WinMMMidiSmoke]::midiInGetDevCaps([UIntPtr]::new([uint64]$i), [ref]$caps, [uint32]$size)
        [pscustomobject]@{
            Direction = "in"
            Id = $i
            Status = $rc
            Name = $caps.szPname
        }
    }
}

$outputs = @(Get-MidiOutputs)
$inputs = @(Get-MidiInputs)

if ($List) {
    $outputs + $inputs | Format-Table -AutoSize
    exit 0
}

$matches = @($outputs | Where-Object { $_.Name -like "*$NameHint*" })
if ($matches.Count -eq 0) {
    Write-Host "No MIDI OUT port matching '$NameHint'. Available ports:"
    $outputs + $inputs | Format-Table -AutoSize
    exit 1
}

$port = $matches[0]
Write-Host "Opening MIDI OUT #$($port.Id): $($port.Name)"

$handle = [IntPtr]::Zero
$rc = [WinMMMidiSmoke]::midiOutOpen([ref]$handle, [uint32]$port.Id, [IntPtr]::Zero, [IntPtr]::Zero, 0)
if ($rc -ne 0) {
    throw "midiOutOpen failed: $rc"
}

try {
    $allOff = New-MidiMessage 0xB0 123 0
    Send-MidiMessage $handle $allOff

    if ($Pattern -eq "scale") {
        foreach ($n in @(60, 62, 64, 65, 67, 69, 71, 72)) {
            Send-Note $handle $n $Velocity $DurationMs
            Start-Sleep -Milliseconds 30
        }
        Write-Host "Sent C major scale."
    } elseif ($Pattern -eq "twinkle") {
        $twinkle = @(
            @(60, 1), @(60, 1), @(67, 1), @(67, 1), @(69, 1), @(69, 1), @(67, 2),
            @(65, 1), @(65, 1), @(64, 1), @(64, 1), @(62, 1), @(62, 1), @(60, 2)
        )
        foreach ($step in $twinkle) {
            Send-Note $handle $step[0] $Velocity ($DurationMs * $step[1])
            Start-Sleep -Milliseconds 30
        }
        Write-Host "Sent Twinkle test phrase."
    } else {
        Send-Note $handle $Note $Velocity $DurationMs
        Write-Host "Sent note $Note velocity $Velocity for ${DurationMs}ms."
    }

    Send-MidiMessage $handle $allOff
} finally {
    if ($handle -ne [IntPtr]::Zero) {
        [void][WinMMMidiSmoke]::midiOutClose($handle)
    }
}
