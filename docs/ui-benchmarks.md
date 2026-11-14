# Native UI benchmark protocol

The [UI catalog](../benchmarks/ui-events.json) covers all 18 catalog entrants on their supported platforms. It defines 24 actions and 72 medal events: action latency, process-tree CPU time, and post-action RSS. The other 45 performance events remain in the same report. Definitions are not measured results.

Implemented collection currently covers idle Media/File, Playback, Audio, Subtitle menus and open-file dialogs where a catalog locator exists. Other actions explicitly need an adapter, geometry collector or presented-frame collector. mpv/FFplay's absent conventional menu bars are inapplicable. Windows Media Player, PotPlayer, Kodi, Plex and custom skins need validated adapters. Missing automation is not evidence of a slow player or missing feature.

Plan safely without launching applications:

```powershell
python scripts/run_ui_benchmarks.py --output results/local/ui-plan
```

Execute on an isolated, logged-in physical desktop with no work in competing applications. Each output directory must be new:

```powershell
python -m pip install pywinauto==0.6.9
python scripts/run_ui_benchmarks.py --execute --players foxide vlc mpc-hc --actions menu_media menu_playback --repeats 10 --environment-label "1920x1080; 100% DPI; dark; en-US; dedicated desktop" --output results/local/ui-windows
```

macOS needs PyObjC's ApplicationServices/Quartz modules and existing Accessibility permission for the runner. Linux needs the distribution's `python3-pyatspi`, an AT-SPI enabled desktop/session bus and a visible display. The runner does not grant permissions or alter security settings. Cross-platform drivers are implemented but not yet validated on native desktops. Coordinate-based click fallbacks are intentionally absent.

Every trial launches a fresh owned process. It refuses existing-instance handoff, checks creation identity around the action, requires the postcondition to be absent first, invokes a native accessibility control, and saves matching owned-control evidence before/after. OS open dialogs hosted by a different process are conservatively rejected until ownership can be demonstrated. Ambiguous names, hidden controls, changed processes, missing permissions and timeouts remain unverified.

Timing runs from immediately before native Invoke/Expand/AX/AT-SPI action to observation of the specific visible result. This includes adapter overhead and 10 ms polling, and is a distinct accessibility-action cohort from physical mouse/key timing. A visible menu item is not proof of video presentation. Seeking, scrubbing, media opening and fullscreen video need content-aware displayed-frame evidence before ranking.

CPU time spans baseline-to-post snapshot collection around the observation window and excludes trials whose observed process tree changes. RSS is the final process-tree sum, not peak or private memory; shared pages can be counted twice. Snapshot overhead differs by OS. Compare only within the same platform, host, action, display/DPI, locale, theme, native backend and window geometry. Current windows retain their default geometry; differing geometry creates separate unranked cohorts rather than a misleading podium. Use fixed clean benchmark profiles and normalize geometry before a competitive run. Do not compare this idle menu suite with playback-state tests.

Generate the existing HTML, SVG and GitHub-native tables with the UI measurements and session alongside playback artifacts:

```powershell
python scripts/build_medal_report.py --measurements results/local/ui-windows/measurements.json --sessions results/local/ui-windows/session.json --output results/local/ui-report
```

The interactive GitHub Actions workflow requires self-hosted runners labeled `foxide-ui` and an existing desktop login. Hosted VM decode benchmarks remain separate: they cannot certify physical presentation, input latency or battery efficiency. Do not install a self-hosted runner on a personal desktop exposed to untrusted pull requests.
