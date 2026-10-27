# Android

The showcase is the only Quinn example that has been installed and drawn on Android. The other examples still lower `view.rs` from the host disk at startup, so they panic on a device. Desktop `--hot` is unchanged.

Run on 27 Oct 2026, about 11:28 AM Asia/Manila. Pixel emulator, Android 15 (API 35), `x86_64`, emulator 37.2.12. The production APK was a release build with `android:debuggable` false. After install, `am start` opened `dev.makepad.showcase` and the Kitchen note was on screen (Library, search, the two paragraphs, and the weekday bars).

## What the phone runs

`examples/showcase/src/main.rs` calls `quinn::load_bundled_view` when `target_os` is `android`. That embeds `examples/showcase/view.rs` and `quinn-ui`'s `components.rs` with `include_str!`, lowers them in memory, and evals the script. The seed window is 1080×1920, so `state.quinn_width` starts at 1080. `sm:` / `md:` / `lg:` apply. `xl:` (1280) does not, and Library opens a menu (Notes, Table, Settings, Gallery) instead of the desktop sidebar.

On every other target the same function is `load_hot_view_with`. It still reads `view.rs`, writes `hot/ui.rs`, and watches that file when the process was started with `--hot`. The seed window stays 1440×900.

`load_hot_view` is what `counter`, `conditional`, `keyed-list`, `form`, `retain`, `tailwind-gallery`, `app-shell`, and `async-dashboard` call. None of those embed `view.rs`. An APK of one of them reaches `eval_hot_file`, does not find `CARGO_MANIFEST_DIR` on the device, and panics.

## Package metadata

`examples/showcase/Cargo.toml` states `edition`, `version`, `license`, and `publish` itself. It does not use `workspace = true` for those fields. `cargo-makepad` copies that manifest into `target/makepad-android-wrapper/showcase/Cargo.toml` and adds a `[workspace]` plus a `[lib]` whose path is `src/main.rs`. The wrapper is its own workspace, so `edition.workspace = true` fails with `workspace.package.edition was not defined`. Cargo then warns that `src/main.rs` is both the lib and the bin. That warning is the wrapper. The desktop `cargo run -p showcase` still builds the bin.

## Toolchain

The pin is Makepad `44ecb822`. Build `cargo-makepad` from that checkout, then run the binary with the Quinn workspace as the current directory. `cargo-makepad` calls `rustup run stable cargo`, so the Android standard libraries have to be on the stable toolchain. Quinn's `rust-toolchain.toml` pin (1.99.0) is not the compiler that produces the `.so`.

This pin's Android flag is `--abi`, not `--target`. `tools/android/README.md` in the Makepad tree still says `--target=all` and `makepad-example-ironfish`. `--target=all` is rejected (`--target=all is not a valid android subcommand`). `makepad-example-ironfish` is not a package in this checkout. ABIs are `aarch64`, `x86_64`, `armv7`, `i686`, and `all`. The default ABI is `aarch64`. The emulator below is `x86_64`.

Install the SDK, NDK, and Rust targets once:

```bash
# from the Makepad checkout
cargo run -p cargo-makepad --release -- android --abi=all toolchain-install
```

That writes `~/.makepad/android_33_linux_x64`: platform 33, build-tools 33.0.1, platform-tools, NDK r28b, a JDK 17, and bundletool. `minSdk` is 26. `targetSdk` is 35. The bundled JDK's `libextnet` is missing; `sdkmanager` for an emulator image should use the system JDK, not `JAVA_HOME` pointed at that tree. `javac` from the bundled JDK did compile the activity.

## Production APK

From the Quinn workspace, with `cargo-makepad` already built:

```bash
unset MAKEPAD_ANDROID_DEBUGGABLE
"$MAKEPAD/target/release/cargo-makepad" android --abi=x86_64 --no-icon build -p showcase --release
```

`--release` is the production profile. The log line is `debuggable=false`, and the manifest attribute `android:debuggable` is `0`. Any other profile is packaged debuggable. `MAKEPAD_ANDROID_DEBUGGABLE=1` forces a release APK to stay debuggable. `--no-icon` skips the check for `examples/showcase/resources/icon_32.png`, `icon_64.png`, `icon_128.png`, and `icon.ico`. Those files are not in the tree. Without the flag the packager prints a warning and continues.

The APK is `target/android/makepad-android-apk/showcase/apk/showcase.apk`. On 27 Oct 2026 it was 42 MB. Package `dev.makepad.showcase`, label `Showcase`, `versionName` `0.1.0`, `versionCode` 1. It is signed with cargo-makepad's bundled debug keystore. That is a release binary, not a Play upload key. `build-aab` is the Play artifact and was not produced.

`android run` installs, starts, and then blocks on `logcat`. `android build` stops at the APK.

```bash
adb install -r target/android/makepad-android-apk/showcase/apk/showcase.apk
adb shell am start -S -n dev.makepad.showcase/dev.makepad.showcase.MakepadApp
```

After that install, `dumpsys package dev.makepad.showcase` listed `versionCode=1`, `minSdk=26`, `targetSdk=35`, `primaryCpuAbi=x86_64`, and flags `HAS_CODE ALLOW_CLEAR_USER_DATA ALLOW_BACKUP LARGE_HEAP`. `DEBUGGABLE` was not among them. The process stayed up.

The font pack logged with the APK is the Makepad widget set: Inter, IBM Plex, LXGW WenKai, Liberation Mono, Noto Color Emoji, and Font Awesome. The showcase shell only names `INTER_FONT_ASSET`.

## Emulator

A Pixel AVD on the API 35 Google APIs `x86_64` image was created with `avdmanager` after `sdkmanager` installed `emulator` and `system-images;android-35;google_apis;x86_64`. Command-line tools used here were `commandlinetools-linux-9862592`.

On this machine nested KVM faulted in `kvm_arch_vcpu_create` (`kernel BUG at arch/x86/kvm/x86.c`). The emulator had to be started with `-accel off`. `-gpu guest` (SwiftShader / swangle) reached the Android home screen, then the showcase died in Makepad's cursor fragment shader: `ERROR: 0:446: '(' : memory exhausted` (`opengl.rs`, `COMPILATION_FAILED`). `-gpu host` logged `VK_ERROR_INCOMPATIBLE_DRIVER` and still presented OpenGL ES through the host driver. That is the run that drew the Kitchen note.

```bash
"$SDK/emulator/emulator" -avd pixel -gpu host -accel off -no-audio -no-boot-anim -no-metrics -memory 2048
```

The software CPU is slow enough that Android showed "System UI isn't responding" while the showcase was starting. The dialog is the system UI. Wait left `dev.makepad.showcase` on screen. A real device with a GPU was not used.
