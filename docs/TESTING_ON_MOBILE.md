# Testing on iOS and Android

`cargo-makepad` builds and installs the examples on a simulator, an emulator, or a device. Install it at the Makepad revision this workspace pins (the `rev` in `crates/makepad/Cargo.toml`):

```bash
cargo install --locked --git https://github.com/makepad/makepad --rev 44ecb822b48106fa76bde4dfd309e5a8a7ff1932 cargo-makepad
```

## iOS simulator

The `aarch64-apple-ios-sim` target must be installed (`rustup target add aarch64-apple-ios-sim`). Boot a simulator first; `run-sim` fails with `No devices are booted` otherwise.

```bash
xcrun simctl boot "iPhone 17 Pro"
xcrun simctl bootstatus "iPhone 17 Pro" -b
open -g -a Simulator
cargo makepad apple ios --org=dev.quinn --app=showcase run-sim -p showcase --release
```

To see the app's `eprintln!` output, build, install, and launch with a console. `SIMCTL_CHILD_` passes an environment variable to the app; `QUINN_JOB_LOG=1` logs the shell, including each status-bar tint change.

```bash
cargo makepad apple ios --org=dev.quinn --app=showcase build -p showcase --release
xcrun simctl install booted target/apple/makepad-apple-app/aarch64-apple-ios-sim/release/showcase.app
SIMCTL_CHILD_QUINN_JOB_LOG=1 xcrun simctl launch --console-pty booted dev.quinn.showcase
```

The simulator shares the Mac's file system, so the app finds `examples/showcase/view.rs` and lowers the current file at startup, as a desktop run does. An emulator or a phone has no source tree: `quinn::load_view_with` takes the view as `include_str!("../view.rs")` and lowers that copy in memory when `view.rs` is not on disk. The other examples call `load_hot_view`, which still needs the file, so they run in the simulator but not on Android or a device yet.

## Android emulator

`cargo makepad android` builds the package inside a wrapper workspace with no `[workspace.package]`, so a manifest that inherits `edition.workspace = true` fails to parse. `examples/showcase/Cargo.toml` spells its package fields out for that reason.

```bash
cargo makepad android --abi=aarch64 install-toolchain
emulator -avd Medium_Phone_API_36 &
adb wait-for-device
cargo makepad android --abi=aarch64 run -p showcase --release
```

`adb exec-out screencap -p > shot.png` takes a screenshot. `adb shell input tap X Y` taps in device pixels. `adb shell input keyevent KEYCODE_BACK` is the system Back.

## What the host does on a phone

- `state.quinn_os` is 1 on iOS and 2 on Android, set at startup. `ios:`, `android:`, and `mobile:` read it. A phone starts at touch density (48 pt controls).
- `state.quinn_width` and `state.quinn_safe_top` / `_right` / `_bottom` / `_left` come from the window geometry, at startup and on each `WindowGeomChange` (rotation). `pt-safe` and the other safe utilities read them.
- A touch that starts outside the focused field clears the focus, as a mouse press does.
- The status bar gets dark icons on a light theme and light icons on a dark one. Makepad's automatic tint reads the window clear color, which Quinn does not paint.
- Android's system Back runs the view's root `on:keydown` handler with `"Escape"`. Back is handled when that handler wrote a signal; otherwise the system takes it and the app goes to the background.
- The Window's body is Makepad's `KeyboardView`, so a focused field is moved above the on-screen keyboard.

## Checklist (showcase)

- The toolbar sits below the status bar and the Dynamic Island. The status icons are dark on the light theme.
- Below 800 pt the library list is the first screen. Tapping Table pushes the Table page, and the toolbar shows `‹ Library` and `Table`. `‹ Library` goes back with Table still selected.
- Theme switches the page to dark and the status icons to light on the same tap.
- Search sits under the toolbar on a phone and beside it from 800 pt up.
- New note opens the sheet inside the safe area. Save closes it and shows the toast.
- Android: Back closes an open sheet first, then goes back from a page to the list.
- A finger drag scrolls a long page (Settings). On the desktop, a drag on content does not scroll; the wheel does.
- A tablet or a landscape phone at 800 pt or more shows the rail and the page side by side, as on the desktop.

## Known gaps

- Android Back at the list does nothing. Quinn leaves the event unhandled there, but this Makepad revision's activity always keeps Back and implements neither `Quit` nor `MinimizeWindow` on Android, so nothing sends the app to the background.
- The Android emulator shows a blank screen for several seconds at startup while the view is lowered on the device.
- The showcase's Density control shows Standard on a phone, which starts at touch density; the three segment signals do not read `quinn_os`.
- On a phone the toast sits over the search row: `Overlay` is placed `top-safe top-[64px]`, and the phone toolbar is taller than 64.
- In the dark theme the sheet's panel is hard to tell from the scrim.
- The page change is instant. Makepad's `StackNavigation` has the iOS slide and the interactive swipe-back; Quinn does not lower to it yet.
- The notes list beside the Notes page is hidden below 1120 pt, so on a phone the Notes page shows the selected note only.
- A text field can show the focus look when its page first appears (the iOS search field at launch, the Settings Folder field on Android), before anything was tapped. A tap elsewhere clears it.
- A trackpad's two-finger scroll does not move a Makepad page in the iOS Simulator: the Simulator sends it as short one-finger touch drags, which Makepad does not treat as a scroll. A click-and-drag (a finger) does scroll. On the Android emulator the same gesture becomes fast swipes that coast too far. Makepad `44ecb822` has no trackpad or mouse-wheel scroll path on iOS or Android.
- The software keyboard was not seen in the simulator, which had a hardware keyboard connected (I/O › Keyboard › Connect Hardware Keyboard toggles it).
