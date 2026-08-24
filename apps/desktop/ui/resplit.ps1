$ErrorActionPreference = "Stop"
$src = "G:\APB AI\apb\apps\desktop\ui\js"
$enc = New-Object System.Text.UTF8Encoding($false)

function Read-Body([string]$name) {
  # drop our generated header (line 1); keep everything else incl. trailing blank
  $all = [System.IO.File]::ReadAllLines("$src\$name", [System.Text.Encoding]::UTF8)
  return $all[1..($all.Count-1)]
}

$core = Read-Body "core.js"     # relative: O1=1-141 O3=142-165 O7=166-204 O13=205-238 (0-based after header strip)
$shell = Read-Body "shell.js"
$tv = Read-Body "tabsview.js"
$hme = Read-Body "home.js"
$panels = Read-Body "panels.js"
$settings = Read-Body "settings.js"
$notes = Read-Body "notes.js"
$graph = Read-Body "graph.js"

function Seg($arr, [int]$from, [int]$to) { return $arr[($from-1)..($to-1)] } # 1-based inclusive

$outLists = @{}

# Each entry: array of (array, from, to) tuples flattened as triples
$plan = @(
  @{ n = "01-dialogs-window-toast.ps1.js";      s = @(,@("core",1,142)) },
  @{ n = "02-sitefx-ctxmenu-find.js";           s = @(,@("shell",1,144)) },
  @{ n = "03-theme-searchengines.js";           s = @(@("core",142,165), @("tv",1,21)) },
  @{ n = "04-profiles-sidepanel.js";            s = @(@("panels",1,75), @("shell",144,180)) },
  @{ n = "05-sound-internalpages.js";           s = @(@("core",166,204), @("shell",181,250)) },
  @{ n = "06-session-ws-downloads-tabs.js";     s = @(,@("tv",22,453)) },
  @{ n = "07-home-widgets.js";                  s = @(,@("home",1,483)) },
  @{ n = "08-omnibox-library-panels.js";        s = @(@("tv",454,476), @("panels",76,1028)) },
  @{ n = "09-notes-editor.js";                  s = @(@("core",205,238), @("notes",1,676)) },
  @{ n = "10-settings-onboarding-pz.js";        s = @(,@("settings",1,379)) },
  @{ n = "11-graph.js";                         s = @(,@("graph",1,$graph.Count)) }
)

$vars = @{ core=$core; shell=$shell; tv=$tv; home=$hme; panels=$panels; settings=$settings; notes=$notes; graph=$graph }

foreach ($p in $plan) {
  $out = New-Object System.Collections.Generic.List[string]
  foreach ($seg in $p.s) {
    $arr = $vars[$seg[0]]
    $from = [int]$seg[1]; $to = [int]$seg[2]
    foreach ($ln in Seg $arr $from $to) { $out.Add($ln) }
  }
  $out.Add("")
  [System.IO.File]::WriteAllText("$src\$($p.n)", ($out -join "`n"), $enc)
  Write-Output ("wrote {0}: {1} lines" -f $p.n, $out.Count)
}

# remove the old thematic files
foreach ($old in @("core.js","shell.js","tabsview.js","home.js","panels.js","settings.js","notes.js","graph.js")) {
  Remove-Item "$src\$old" -Force
}
Write-Output "old files removed"

