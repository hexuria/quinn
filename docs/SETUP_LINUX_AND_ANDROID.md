# Linux and Android setup

Linux is the host this workspace was run on. Android builds go through Makepad revision `44ecb822b48106fa76bde4dfd309e5a8a7ff1932`, the pin in `crates/makepad/Cargo.toml`. Quinn's example windows were run on Linux. They were not packaged as Android apps. The Android section built Makepad's `makepad-example-counter` from that checkout and installed it on an emulator.

macOS is a separate checklist: [TESTING_ON_MACOS.md](TESTING_ON_MACOS.md). Leave `MAKEPAD_GPU` and `LIBGL_ALWAYS_SOFTWARE` unset there.

## Linux host

These package names are Debian and Ubuntu. They cover the libraries the native Makepad window links: X11, the cursor, GL and EGL, Wayland, ALSA, PulseAudio, and OpenSSL. `libgl1-mesa-dri` supplies llvmpipe. `xvfb` is the headless display.

```bash
sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  pkg-config \
  libx11-dev \
  libxcursor-dev \
  libxkbcommon-dev \
  libasound2-dev \
  libgl1-mesa-dev \
  libegl1-mesa-dev \
  libwayland-dev \
  libpulse-dev \
  libssl-dev \
  libgl1-mesa-dri \
  mesa-utils \
  xvfb
```

`rust-toolchain.toml` pins channel 1.99.0, profile `minimal`, components `rustfmt` and `clippy`, and the `wasm32-unknown-unknown` target. With rustup on `PATH`, entering the workspace selects that toolchain. On a fresh machine:

```bash
rustup toolchain install 1.99.0 --profile minimal --component rustfmt,clippy
rustup default 1.99.0
rustup target add wasm32-unknown-unknown --toolchain 1.99.0
```

Headless tests, from the workspace root. No GPU and no display:

```bash
cargo test -p quinn-styles -p quinn-reactive -p quinn-core -p quinn-lower -p quinn --no-default-features
```

That command passed 54 tests on the Linux x86_64 host used for this note.

A window, software OpenGL under a real `DISPLAY`:

```bash
export MAKEPAD_GPU=gl
export LIBGL_ALWAYS_SOFTWARE=1
cargo run -p counter --release
```

Without a display, `xvfb-run -a cargo run -p counter --release` starts one. The same two variables and the other example names are in the README section "Run a native example", including `--hot`.

## Android toolchain

`cargo-makepad` is not a member of this workspace. It is a package in the Makepad checkout Cargo fetches for `makepad-widgets`. From the workspace root, after `cargo fetch` or any command that resolves that git dependency:

```bash
MAKEPAD="$(find "${CARGO_HOME:-$HOME/.cargo}/git/checkouts" -maxdepth 2 -type d -name '44ecb82' -print -quit)"
cd "$MAKEPAD"
```

`44ecb82` is the first seven hex digits of the pin. The rest of the Android commands run in that directory, not in the Quinn workspace. `cargo run -p cargo-makepad` from the Quinn root has no such package.

That checkout does not carry Quinn's `rust-toolchain.toml`. Compile `cargo-makepad` with 1.99 (`cargo +1.99.0`). An older default such as 1.83 stops on `edition2024`.

### Flags at this revision

```bash
cargo +1.99.0 run -p cargo-makepad --release -- android help
```

The ABI option is `--abi=aarch64|x86_64|armv7|i686|all`. The default is `aarch64`. This android subcommand has no `--target` flag.

`makepad-example-ironfish` is not in this checkout (`examples/ironfish` was removed). The small example that is still there is `examples/counter`, package name `makepad-example-counter`.

`--release` before `android` builds the `cargo-makepad` binary. The app's release flag goes after the subcommand (`build --release`, `run --release`). `--no-icon` is forwarded into `rustc` and the app build fails. Leave it off. A missing-icon warning on its own is non-fatal.

### Install

`toolchain-install` downloads its own OpenJDK 17.0.2. The emulator steps later also want `unzip`, `curl`, and `ca-certificates` on the host.

```bash
cargo +1.99.0 run -p cargo-makepad --release -- android --abi=all toolchain-install
```

On Linux x64 that command:

- installs the `stable` Rust toolchain when it is missing, without force-updating it
- adds `aarch64-linux-android`, `x86_64-linux-android`, `armv7-linux-androideabi`, and `i686-linux-android` to `stable`
- writes the SDK to `$HOME/.makepad/android_33_linux_x64`

The SDK from this revision contains platform-tools (adb 33.0.3), build-tools 33.0.1, platform `android-33-ext4`, NDK r28b at `ndk/28.2.13676358`, and OpenJDK 17.0.2 at `openjdk`. The tree is several gigabytes. App compiles call `rustup run stable`, so the Android targets belong on `stable`. The workspace toolchain 1.99.0 can stay without them.

`--abi=all` is for this install. A later `build` or `run` should name the ABI of the device in front of you.

### Emulator

`toolchain-install` does not install an emulator or a system image. The image below is Android 14 (`android-34`), Google APIs, `x86_64`. The matching app ABI is `x86_64`.

```bash
SDK="$HOME/.makepad/android_33_linux_x64"
export JAVA_HOME="$SDK/openjdk"
export ANDROID_SDK_ROOT="$SDK"
export ANDROID_HOME="$SDK"
export PATH="$JAVA_HOME/bin:$SDK/platform-tools:$SDK/emulator:$PATH"
```

`JAVA_HOME` points at the JDK the toolchain just unpacked. The distro `java` can be a newer release and still disagree with `sdkmanager`.

Command-line tools, then the emulator and the system image. The zip used here was `commandlinetools-linux-13114758_latest.zip`. `sdkmanager --licenses` piped from `yes` can exit 141 (`SIGPIPE`); `|| true` lets the next command run. Accept the licenses in a terminal if you would rather see each prompt.

```bash
tmp="$(mktemp -d)"
curl -fL --retry 3 -o "$tmp/cmdtools.zip" \
  https://dl.google.com/android/repository/commandlinetools-linux-13114758_latest.zip
mkdir -p "$SDK/cmdline-tools"
unzip -q "$tmp/cmdtools.zip" -d "$tmp"
rm -rf "$SDK/cmdline-tools/latest"
mv "$tmp/cmdline-tools" "$SDK/cmdline-tools/latest"
rm -rf "$tmp"

yes | "$SDK/cmdline-tools/latest/bin/sdkmanager" --sdk_root="$SDK" --licenses || true
"$SDK/cmdline-tools/latest/bin/sdkmanager" --sdk_root="$SDK" \
  "emulator" "system-images;android-34;google_apis;x86_64"

echo no | "$SDK/cmdline-tools/latest/bin/avdmanager" create avd \
  -n makepad \
  -k "system-images;android-34;google_apis;x86_64" \
  -d pixel_6 \
  --force
```

Write `$HOME/.android/advancedFeatures.ini`:

```
Uwb = off
DownloadableSnapshot = off
```

In `$HOME/.android/avd/makepad.avd/config.ini` set:

```
hw.ramSize = 2048
hw.keyboard = yes
fastboot.forceColdBoot = yes
firstboot.bootFromDownloadableSnapshot = no
firstboot.bootFromLocalSnapshot = no
firstboot.saveToLocalSnapshot = no
```

`hardware-qemu.ini` is rewritten when the emulator starts. Those keys belong in `config.ini`.

Start the emulator in its own terminal and leave that process up. A background job in a shell that then exits can take the emulator with it.

```bash
"$SDK/emulator/emulator" -avd makepad \
  -no-audio -no-boot-anim -no-metrics \
  -gpu swiftshader_indirect -accel on -no-snapshot \
  -memory 2048 -partition-size 2048
```

Add `-no-window` on a host with no display. `-accel on` is the setting to use when `/dev/kvm` can create a guest vCPU. If `/dev/kvm` exists and is not writable, `sudo chmod 666 /dev/kvm`, or add your user to the `kvm` group when that group exists.

On a nested virtual machine running Linux 6.12.94, `emulator -accel-check` reported KVM as installed and usable, and the guest still never reached boot. The host kernel logged `kernel BUG at arch/x86/kvm/x86.c:702` inside `kvm_arch_vcpu_create`. The boot that finished used `-accel off` (TCG, software CPU). Cold boot took a few minutes: about 190 seconds on one run, and `Boot completed in 220282 ms` on another. TCG prints that it does not support AVX or F16C.

Wait for the guest:

```bash
adb start-server
adb -s emulator-5554 shell getprop sys.boot_completed
adb -s emulator-5554 shell getprop ro.build.version.sdk
adb -s emulator-5554 shell getprop ro.product.cpu.abi
```

`sys.boot_completed` prints `1` when the guest is ready. The image checked here printed sdk `34` and abi `x86_64`. Pass one property per `getprop`. A second name in the same call is an error.

A leftover `adb connect 127.0.0.1:5555` next to `emulator-5554` makes adb report more than one device. `adb disconnect 127.0.0.1:5555` leaves the emulator.

### Run Makepad's counter

The emulator (or a handset) has to already be online. `android run` does not boot an AVD. From the Makepad checkout:

```bash
cargo +1.99.0 run -p cargo-makepad --release -- \
  android --abi=x86_64 run -p makepad-example-counter --release
```

`run` installs the APK, starts `dev.makepad.makepad_example_counter/dev.makepad.makepad_example_counter.MakepadApp`, then stays on `adb logcat --pid`. Stop that process when you have the log. The app stays installed.

A build that exits, and does not install:

```bash
cargo +1.99.0 run -p cargo-makepad --release -- \
  android --abi=x86_64 build --release -p makepad-example-counter
```

The APK is under `target/android/` in the Makepad checkout. The release APK from this check was about 41 MB.

On 27 Oct 2026 (Asia/Manila) that release activity came up on the API 34 x86_64 emulator at 1080×2400 and drew a large `0` and an Increment control. The first capture, taken before the first frame, was black. A later `adb shell input tap` did not change the framebuffer, so the Increment click was not confirmed.

### A handset

Turn on Developer options and USB debugging. `adb devices` lists the serial once the cable or `adb tcpip` session is up. Current phones are `arm64-v8a`, which is `--abi=aarch64`. Pass `--devices=<serial>` when more than one device is attached. `run` still blocks on logcat, and it still expects the device to be online before it starts.

```bash
cargo +1.99.0 run -p cargo-makepad --release -- \
  android --abi=aarch64 --devices=<serial> run -p makepad-example-counter --release
```

A handset was not used for the check above. That run was the x86_64 emulator, so the ABI was `x86_64`.

## What this does not cover

Quinn's `examples/counter` and the other workspace binaries were not built as APKs. [compatibility.md](compatibility.md) records that Quinn's Android UI has not been executed. `quinn-makepad` links the same Makepad revision, which is why the toolchain above is the one that matches this repo.
