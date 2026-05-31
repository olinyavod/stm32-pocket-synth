param(
    [string]$NameHint = "Pocket Synth",
    [ValidateSet("twinkle", "scale", "tetris", "mario", "imperial", "imperial-long", "all")]
    [string]$Song = "all",
    [int]$Velocity = 100,
    [ValidateSet("melody", "full")]
    [string]$Arrangement = "full",
    [int]$EventGapMs = 5,
    [switch]$List
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public static class WinMMBuiltinPlayer {
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
    $rc = [WinMMBuiltinPlayer]::midiOutShortMsg($Handle, $Message)
    if ($rc -ne 0) {
        throw "midiOutShortMsg failed: $rc"
    }
}

function Get-MidiOutputs {
    $count = [WinMMBuiltinPlayer]::midiOutGetNumDevs()
    $size = [Runtime.InteropServices.Marshal]::SizeOf([type][WinMMBuiltinPlayer+MIDIOUTCAPS])
    for ($i = 0; $i -lt $count; $i++) {
        $caps = New-Object WinMMBuiltinPlayer+MIDIOUTCAPS
        $rc = [WinMMBuiltinPlayer]::midiOutGetDevCaps([UIntPtr]::new([uint64]$i), [ref]$caps, [uint32]$size)
        [pscustomobject]@{
            Id = $i
            Status = $rc
            Name = $caps.szPname
        }
    }
}

function Open-MidiOutput([string]$Hint) {
    $outputs = @(Get-MidiOutputs)
    if ($List) {
        $outputs | Format-Table -AutoSize
        exit 0
    }

    $matches = @($outputs | Where-Object { $_.Name -like "*$Hint*" })
    if ($matches.Count -eq 0) {
        Write-Host "No MIDI OUT port matching '$Hint'. Available ports:"
        $outputs | Format-Table -AutoSize
        exit 1
    }

    $port = $matches[0]
    Write-Host "Opening MIDI OUT #$($port.Id): $($port.Name)"
    $handle = [IntPtr]::Zero
    $rc = [WinMMBuiltinPlayer]::midiOutOpen([ref]$handle, [uint32]$port.Id, [IntPtr]::Zero, [IntPtr]::Zero, 0)
    if ($rc -ne 0) {
        throw "midiOutOpen failed: $rc"
    }
    return $handle
}

function Send-AllOff([IntPtr]$Handle) {
    Send-MidiMessage $Handle (New-MidiMessage 0xB0 123 0)
}

function Add-TrackEvents([System.Collections.ArrayList]$Events, [array]$Track, [int]$UnitMs, [int]$Channel) {
    $time = 0
    $gap = 18
    foreach ($step in $Track) {
        $note = [int]$step[0]
        $units = [int]$step[1]
        $duration = [Math]::Max(1, $UnitMs * $units)
        if ($note -ne 0) {
            [void]$Events.Add([pscustomobject]@{ Time = $time; Status = 0x90 + $Channel; Note = $note; Velocity = $Velocity })
            [void]$Events.Add([pscustomobject]@{ Time = [Math]::Max($time + 1, $time + $duration - $gap); Status = 0x80 + $Channel; Note = $note; Velocity = 0 })
        }
        $time += $duration
    }
}

function Play-Tracks([IntPtr]$Handle, [array[]]$Tracks, [int]$UnitMs) {
    $events = New-Object System.Collections.ArrayList
    for ($i = 0; $i -lt $Tracks.Count; $i++) {
        Add-TrackEvents $events $Tracks[$i] $UnitMs $i
    }

    $ordered = $events | Sort-Object Time, Status
    $last = 0
    foreach ($event in $ordered) {
        $delay = [int]$event.Time - $last
        if ($delay -gt 0) {
            Start-Sleep -Milliseconds $delay
        }
        Send-MidiMessage $Handle (New-MidiMessage $event.Status $event.Note $event.Velocity)
        if ($EventGapMs -gt 0) {
            Start-Sleep -Milliseconds $EventGapMs
        }
        $last = [int]$event.Time
    }
}

$Twinkle = @(
    @(60, 1), @(60, 1), @(67, 1), @(67, 1), @(69, 1), @(69, 1), @(67, 2),
    @(65, 1), @(65, 1), @(64, 1), @(64, 1), @(62, 1), @(62, 1), @(60, 2),
    @(67, 1), @(67, 1), @(65, 1), @(65, 1), @(64, 1), @(64, 1), @(62, 2),
    @(67, 1), @(67, 1), @(65, 1), @(65, 1), @(64, 1), @(64, 1), @(62, 2),
    @(60, 1), @(60, 1), @(67, 1), @(67, 1), @(69, 1), @(69, 1), @(67, 2),
    @(65, 1), @(65, 1), @(64, 1), @(64, 1), @(62, 1), @(62, 1), @(60, 2)
)

$Scale = @(
    @(60, 1), @(62, 1), @(64, 1), @(65, 1), @(67, 1), @(69, 1), @(71, 1), @(72, 2)
)

$TetrisMelody = @(
    @(76, 2), @(71, 1), @(72, 1), @(74, 2), @(72, 1), @(71, 1),
    @(69, 2), @(69, 1), @(72, 1), @(76, 2), @(74, 1), @(72, 1),
    @(71, 3), @(72, 1), @(74, 2), @(76, 2),
    @(72, 2), @(69, 2), @(69, 2), @(0, 2),
    @(74, 3), @(77, 1), @(81, 2), @(79, 1), @(77, 1),
    @(76, 3), @(72, 1), @(76, 2), @(74, 1), @(72, 1),
    @(71, 2), @(71, 1), @(72, 1), @(74, 2), @(76, 2),
    @(72, 2), @(69, 2), @(69, 2), @(0, 2)
)

$TetrisBass = @(
    @(45, 4), @(52, 4), @(45, 4), @(52, 4),
    @(40, 4), @(47, 4), @(45, 4), @(52, 4),
    @(38, 4), @(45, 4), @(48, 4), @(43, 4),
    @(45, 4), @(52, 4), @(45, 4), @(52, 4)
)

$MarioMelody = @(
    @(76, 1), @(76, 1), @(0, 1), @(76, 1), @(0, 1), @(72, 1), @(76, 1), @(0, 1),
    @(79, 1), @(0, 3), @(67, 1), @(0, 3),
    @(72, 1), @(0, 2), @(67, 1), @(0, 2), @(64, 1), @(0, 2),
    @(69, 1), @(0, 1), @(71, 1), @(0, 1), @(70, 1), @(69, 1), @(0, 1),
    @(67, 1), @(76, 1), @(79, 1), @(81, 1), @(0, 1), @(77, 1), @(79, 1),
    @(0, 1), @(76, 1), @(0, 1), @(72, 1), @(74, 1), @(71, 1), @(0, 2)
)

$MarioBass = @(
    @(48, 8), @(43, 8), @(48, 8), @(45, 4), @(43, 4), @(53, 4), @(48, 4), @(50, 4), @(43, 4)
)

$ImperialMelody = @(
    @(67, 2), @(67, 2), @(67, 2),
    @(63, 1), @(0, 1), @(70, 1), @(0, 1), @(67, 2),
    @(63, 1), @(0, 1), @(70, 1), @(0, 1), @(67, 4),
    @(74, 2), @(74, 2), @(74, 2),
    @(75, 1), @(0, 1), @(70, 1), @(0, 1), @(66, 2),
    @(63, 1), @(0, 1), @(70, 1), @(0, 1), @(67, 4)
)

$ImperialBass = @(
    @(43, 6), @(39, 2), @(46, 2), @(43, 2), @(39, 2), @(46, 2), @(43, 4),
    @(50, 6), @(51, 2), @(46, 2), @(42, 2), @(39, 2), @(46, 2), @(43, 4)
)

$ImperialBridgeMelody = @(
    @(67, 2), @(0, 1), @(67, 1), @(67, 2), @(66, 1), @(65, 1),
    @(64, 2), @(63, 2), @(64, 2), @(0, 2),
    @(70, 2), @(0, 1), @(70, 1), @(70, 2), @(69, 1), @(68, 1),
    @(67, 2), @(66, 2), @(67, 4)
)

$ImperialBridgeBass = @(
    @(43, 4), @(50, 4), @(43, 4), @(51, 4),
    @(46, 4), @(42, 4), @(43, 8)
)

$ImperialFinalMelody = @(
    @(74, 2), @(74, 2), @(74, 2), @(75, 1), @(0, 1), @(70, 1), @(0, 1),
    @(67, 2), @(63, 2), @(70, 2), @(67, 6), @(0, 2),
    @(67, 1), @(67, 1), @(67, 1), @(67, 1), @(67, 8)
)

$ImperialFinalBass = @(
    @(50, 6), @(51, 2), @(46, 2), @(43, 2), @(39, 2), @(46, 2),
    @(43, 8), @(31, 4), @(43, 8)
)

$ImperialLongMelody = $ImperialMelody + $ImperialBridgeMelody + $ImperialMelody + $ImperialFinalMelody
$ImperialLongBass = $ImperialBass + $ImperialBridgeBass + $ImperialBass + $ImperialFinalBass

$Library = @{
    twinkle = @{ Tracks = [object[]]@(,$Twinkle); UnitMs = 135 }
    scale = @{ Tracks = [object[]]@(,$Scale); UnitMs = 130 }
    tetris = @{ Tracks = ([object[]]@(,$TetrisMelody) + [object[]]@(,$TetrisBass)); UnitMs = 145 }
    mario = @{ Tracks = ([object[]]@(,$MarioMelody) + [object[]]@(,$MarioBass)); UnitMs = 115 }
    imperial = @{ Tracks = ([object[]]@(,$ImperialMelody) + [object[]]@(,$ImperialBass)); UnitMs = 180 }
    "imperial-long" = @{ Tracks = ([object[]]@(,$ImperialLongMelody) + [object[]]@(,$ImperialLongBass)); UnitMs = 170 }
}

$SongsToPlay = if ($Song -eq "all") {
    @("twinkle", "scale", "tetris", "mario", "imperial")
} else {
    @($Song)
}

$handle = Open-MidiOutput $NameHint
try {
    Send-AllOff $handle
    foreach ($name in $SongsToPlay) {
        $entry = $Library[$name]
        $tracks = $entry.Tracks
        if ($Arrangement -eq "melody" -and $tracks.Count -gt 1) {
            $tracks = [object[]]@(,$tracks[0])
        }

        Write-Host "Playing: $name ($Arrangement)"
        Play-Tracks $handle $tracks $entry.UnitMs
        Send-AllOff $handle
        Start-Sleep -Milliseconds 350
    }
    Write-Host "Done."
} finally {
    if ($handle -ne [IntPtr]::Zero) {
        Send-AllOff $handle
        [void][WinMMBuiltinPlayer]::midiOutClose($handle)
    }
}
