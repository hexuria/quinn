# Building one app for macOS, iOS, and Android

A Quinn app is one `view.rs` and one component library. The same file runs on the desktop (macOS, Linux, Windows), in the iOS simulator, and on Android. What changes from device to device is decided at runtime from a few host facts, written as class variants and signals in that one file. There is no per-platform view file and no `cfg` in `view.rs`: the lowerer reads it as authoring source, not compiled Rust.

This guide covers how to run each target and how to organize views and components so they adapt. The utility reference is [compatibility.md](compatibility.md); the build details are [TESTING_ON_MACOS.md](TESTING_ON_MACOS.md), [TESTING_ON_MOBILE.md](TESTING_ON_MOBILE.md), and [ANDROID.md](ANDROID.md).

## Running each target

| Target | Command | Notes |
|---|---|---|
| macOS (and Linux, Windows) | `cargo run -p showcase --release` | Add `-- --hot` to reload `view.rs` on save. |
| iOS simulator | `cargo makepad apple ios --org=dev.quinn --app=showcase run-sim -p showcase --release` | Boot a simulator first. The simulator reads `view.rs` from the Mac. |
| Android emulator or phone | `cargo makepad android --abi=aarch64 run -p showcase --release` | Package `dev.makepad.showcase`. Use `--abi=x86_64` for an x86 emulator. |

Install `cargo-makepad` at the Makepad revision the workspace pins; [TESTING_ON_MOBILE.md](TESTING_ON_MOBILE.md) has the command and the simulator and emulator setup. An iOS device build (`run-device`) needs a signing profile and has not been run.

### The entry point

A phone has no source tree, so `main.rs` embeds the view and lets the host choose:

```rust
fn script_mod(vm: &mut ScriptVm) -> ScriptValue {
    quinn::makepad_widgets::script_mod(vm);
    let component = Self::script_component(vm).script_to_value(vm);
    quinn::load_view_with(
        vm,
        env!("CARGO_MANIFEST_DIR"),
        module_path!(),
        "Notes",
        1440,
        900,
        component,
        quinn_ui::SOURCE,
        include_str!("../view.rs"),
    )
}
```

Where `view.rs` is on disk (desktop, iOS simulator), this is `load_hot_view_with`: it lowers the file, writes `hot/ui.rs`, and watches it under `--hot`. Where it is not (Android, a device), it lowers the embedded copy in memory with `load_bundled_view`. The 1440×900 size is only the first frame's seed; on a phone the host replaces it with the screen size at startup. The other examples still call `load_hot_view`, which needs the file, so they do not start on Android yet.

The app's `Cargo.toml` must spell out `version`, `edition`, and `license` instead of inheriting them from the workspace, because `cargo makepad android` builds it inside a wrapper workspace.

## What the host tells the view

`drive_shell` keeps these in script state. Every variant below reads them, and they update on rotation, resize, and theme change.

| State | Set from | Read by |
|---|---|---|
| `quinn_width` | window width in points, at startup and on each resize or rotation | `sm:` `md:` `lg:` `xl:` `bp800:` `bp1120:` `bp1440:`, and `max-` in front of any of them |
| `quinn_os` | `cx.os_type()`: 0 desktop, 1 iOS, 2 Android | `ios:` `android:` `mobile:` |
| `quinn_safe_top` / `_right` / `_bottom` / `_left` | the window's safe-area insets (0 on desktop) | `p-safe` `px-safe` `pt-safe` … and `top-safe` … |
| `quinn_dark` | the app's theme signal | `dark:`; the host also sets the phone's status-bar icons from it |
| `quinn_density` | 2 (touch, 48 pt controls) on a phone, the app's choice on desktop | control heights |

The host also forwards platform input:

- A touch behaves like a press: a touch outside the focused field clears its focus.
- Android's system Back runs the root `on:keydown` handler with `"Escape"`. It counts as handled only if the handler wrote a signal. Otherwise the system gets it.
- Scroll views follow a finger drag on iOS and Android, and the mouse wheel and trackpad on desktop.

## Decide by size first, platform second

Most of what differs between a phone and a Mac is space, not operating system. An iPad in landscape wants the desktop layout, and a narrow Mac window wants the phone one. So:

1. **Width (`max-bp800:`, `bp800:`, `bp1120:`)** chooses the layout: one column or a split view, a rail or a list, a toolbar row or two.
2. **Safe-area utilities** keep content clear of the notch, the status bar, and the home indicator. They are 0 on desktop, so add them unconditionally to the edges that touch the screen.
3. **Platform (`ios:`, `android:`, `mobile:`)** is for the few things that really belong to one system: a platform's conventions for spacing or labels, or working around a platform gap. If you reach for `mobile:` to change the layout, you probably want `max-bp800:`.

Pick one breakpoint as the app's compact boundary and use it everywhere. The showcase uses 800: below it the app is one screen at a time, and from 800 up it is a split view. Mixing `md:` (768) in one place and `bp800:` in another leaves a band of widths where half the app is compact.

## Organizing the view

### Components adapt themselves

A component should look right at every size without the caller passing a mode. Put its compact and regular looks in its own classes, so every call site gets them. The showcase's `NavItem` is an icon over a caption in the 64-pt rail, a row from 1120 up, and a full-width list row below 800:

```rust
#[component]
fn NavItem(#[prop(into)] icon: String, #[prop(optional, default = false)] selected: bool, children: Children) -> impl IntoView {
    view! {
        <button widget="QNavItem" class="w-full h-[56px] flex flex-col items-center justify-center gap-1 text-caption
            bp1120:flex-row bp1120:justify-start bp1120:h-fit bp1120:py-2 bp1120:px-3 bp1120:gap-2 bp1120:text-body
            max-bp800:flex-row max-bp800:justify-start max-bp800:h-fit max-bp800:py-3 max-bp800:px-4 max-bp800:gap-3 max-bp800:text-body">
            {children()}
        </button>
    }
}
```

Keep look and size in the component library (`crates/ui/components.rs`, or your app's own library string). Keep navigation and which screen is showing in `view.rs`.

### Screens: state in signals, layout in variants

Which screen is on top is app state, so keep it in signals. The layout reads those signals only where it is compact. The showcase's split view uses two signals:

```rust
let (at_root, set_at_root) = signal(true);     // the list is showing
let (page_open, set_page_open) = signal(false); // a page has been pushed
```

- The list is full width on a phone and a rail from 800 up. On a phone it hides once a page is open: `class:max-bp800:hidden=move || page_open.get()`.
- Each page hides on a phone while the list is showing: `class:max-bp800:hidden=move || at_root.get()`.
- From 800 up neither condition applies, so both show side by side and the signals have no effect.
- Choosing a list row sets `page_open` and clears `at_root`. A `‹ Back` button, shown only below 800 (`bp800:hidden`) and only off the root, does the reverse.

`class:` takes the same variant prefixes as `class`, so `class:max-bp800:hidden=move || flag.get()` reads "hidden when the flag is set and the window is compact". That pairing of a signal with a size variant is the main tool for adaptive navigation.

### Show one version of a control, not two layouts

When a control moves between sizes, such as Search sitting beside the toolbar on desktop and on its own row on a phone, render it twice with opposite variants (`max-bp800:hidden` and `bp800:hidden`) and bind both copies to the same signal. Don't build two whole view trees. Duplicate only the pieces that move.

`hidden` takes no space, so a hidden copy doesn't leave a gap. A component's call-site `class` cannot carry `hidden` today; wrap the component in a `div` and put the variant on the div:

```rust
<div class="flex flex-row w-fit h-fit max-bp800:hidden">
    <Button variant="ghost" on:click=move |_| set_nav_open.set(true)>"Library"</Button>
</div>
```

### Safe areas

Pad the edges that touch the screen, not every container:

- The top bar: `pt-safe px-safe`. Its inner row keeps its own height (`h-toolbar`), so the bar grows by the inset.
- The body row: `px-safe`, for a landscape phone's notch.
- A scrolling page: `pb-safe` on the scroll view, so the last row scrolls clear of the home indicator.
- Overlays and sheets: `p-safe p-6` on the full-screen container, `top-safe top-[64px]` on a popover under the toolbar.

A safe utility adds to the base pad on the same side: `pt-safe pt-2` is the inset plus 8.

### Back and Escape in one handler

Give the root `<div>` one `on:keydown` that closes the topmost thing first. The desktop gets it from the Escape key, and Android gets it from system Back:

```rust
<div class="flex flex-col h-full" on:keydown=move |ev| {
    if ev.key() == "Escape" {
        if sheet.get() { set_sheet.set(false); }
        else if nav_open.get() { set_nav_open.set(false); }
        else if page_open.get() { set_page_open.set(false); set_at_root.set(true); }
    }
}>
```

Order it like a stack: dialogs, then popovers, then pushed pages. When no branch writes a signal, Back is left to the system.

### Touch and pointer

- Hit targets: a phone starts at touch density (48 pt controls). Don't hard-code a smaller control height for compact layouts.
- `hover:` is ignored on touch, so make sure every hover-only cue also has a pressed or selected state.
- Don't put anything a user needs behind a right-click or a keyboard shortcut alone.

## A checklist per change

Run each layout change at these four sizes before calling it done:

1. A desktop window at 1440 wide, and the same window dragged below 800 (compact on desktop, no safe area).
2. iPhone simulator, portrait: safe area at top and bottom, the compact layout, finger-drag scrolling.
3. Android emulator: the same, plus system Back closing overlays and popping pages.
4. A landscape phone or an iPad at 800 or more: the split layout, with left and right safe areas.

## Not supported yet

- Page changes are instant. Makepad's `StackNavigation` (the iOS slide and the swipe-back gesture) is not lowered.
- On Android, Back on the first screen does nothing: this Makepad revision never sends the app to the background.
- A trackpad's two-finger scroll does not scroll in the iOS simulator. A finger drag does, on the simulator and on a device.
- There is no per-platform component file or `cfg` in `view.rs`. Everything that differs is a variant or a signal.

[TESTING_ON_MOBILE.md](TESTING_ON_MOBILE.md) lists the remaining known gaps.
