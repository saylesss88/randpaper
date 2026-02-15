[![Nix Flake](https://img.shields.io/badge/Nix_Flake-Geared-dddd00?logo=nixos&logoColor=white)](https://nixos.org/manual/nix/stable/command-ref/new-cli/nix3-flake.html)

[![Nix](https://img.shields.io/badge/Nix-5277C3?style=flat&logo=nixos&logoColor=white)](https://nixos.org)

<video src="https://github.com/user-attachments/assets/e0ad3f8a-9d5a-4229-a797-cd756817762d" controls></video>

# randpaper - A high-performance wallpaper & theme daemon for Wayland

`randpaper` is a lightweight Rust utility designed for efficiency. It manages
per-monitor wallpaper rotation and optional system-wide theme
synchronization—replacing complex script chains with a single, optimized binary.

# 🚀 Features

- ⚡ **Performance**: Written in Rust using `tokio`. Scans your image directory
  once into memory (caching paths) to prevent disk I/O spikes, even with massive
  wallpaper collections.

- 🎨 **Dynamic Theming**: Automatically extracts dominant colors from the
  current wallpaper and generates config files for **Ghostty**, **Kitty**, and
  **Foot**, keeping your terminal in sync with your desktop.
  - 🫟 **Waybar Theming**: Generates CSS variables based on the image palette.
    and reloads Waybar when it changes wallpapers applying the theming
    automatically.

- 🖥️ **Multi-Monitor**: Assigns a unique random image to every active output
  simultaneously.

- 🔄 **Dual Operating Modes**: Run as a background daemon with automatic
  cycling, or use one-shot mode for manual wallpaper changes.

- 🛠️ **Modular Backends**: Works seamlessly with **Sway** and **Hyprland** (via
  Sway IPC or `hyprctl`) and supports both `swaybg` and `swww` renderers.

- Support for both `swww` and its replacement `awww`.

---

## 🛠 Installation

**Prerequisites**

You need `swaybg` or (`swww` / `awww`) installed as the renderer.

```bash
# From source
cargo install --git https://github.com/saylesss88/randpaper
# crates.io
cargo install randpaper
```

<details>
<summary> ✔️ Nix </summary>

Add as a flake input:

```nix
inputs = {
  randpaper.url = "github:saylesss88/randpaper";
};
```

Install in `environment.systemPackages` or `home.packages`:

```nix
environment.systemPackages = {
  inputs.randpaper.packages.${pkgs.stdenv.hostPlatform.system}.default
};
```

- Pass `inputs` through `specialArgs` in your `flake.nix`

And add an `exec` for either hyprland or sway, only for `randpaper`:

Hyprland Example:

```nix
wayland.windowManager.hyprland = {
  settings = {
    exec-once = [
  # Standard Usage
  "randpaper --time 15m /home/your-user/Pictures/wallpapers --backend hyprland --renderer swww"
  # UWSM Usage w/ swaybg
  "uwsm app -- randpaper --time 15m /home/your-user/Pictures/wallpapers --backend hyprland --renderer swaybg"
    ];
  };
}
```

Sway Example:

```nix
wayland.windowManager.sway = {
  extraConfig = ''
    # Default backend is Sway
     exec randpaper --time 30m /home/your-user/wallpapers
  '';
};
```

> Note: `randpaper` manages the renderer process for you. You do not need
> separate `exec-once = swaybg ...` or `exec-once = swww ...` lines in your
> config.

</details>

---

## 🧾 Usage

**Daemon Mode (Background Process)**

With `swww` or `awww` installed you can test which mode you want from the
command line before adding an `exec-once` for `randpaper` to your configuration:

When you provide the `--time` flag, `randpaper` runs as a daemon and
automatically cycles wallpapers at the specified interval:

```bash
# Change every 5 minutes using Hyprland + swww transitions
randpaper --time 5m --backend hyprland --renderer swww ~/Pictures/wallpapers

# Change every hour with custom transitions
randpaper --time 1h --renderer swww --transition-type fade ~/Pictures/wallpapers
```

- These commands work without `swww-daemon` running because `randpaper`
  automatically launches a `swww-daemon` process if one isn't already running.

**One-Shot Mode (Pick Once & Exit)**

Without the `--time` flag, `randpaper` picks a random wallpaper, updates themes,
and exits immediately.

It's recommended to run one-shot via a compositor keybind so it inherits the
correct session/IPC environment.(i.e., running
`randpaper ~/Pictures/wallpapers`from the command line doesn't work as expected)

```conf
# Sway: cycle wallpaper + themes now (one-shot)
bindsym $mod+Shift+n exec randpaper ~/Pictures/wallpapers
```

```conf
# Hyprland: cycle wallpaper + themes (one-shot)
"$mod SHIFT,N,exec, randpaper --backend hyprland /home/jr/Pictures/wallpapers2"
```

- `swaybg` can't be one-shot from the command line because it's not a
  set-and-exit command; it's a long-running wallpaper service.

**Use cases for one-shot mode**:

- Manual wallpaper changes via keybinds

- Scripted theme updates

- Testing without running a daemon

**swww/awww Transitions**:

```bash
# Use fade transitions
randpaper --renderer swww --transition-type fade --transition-step 90 --transition-fps 60 ~/Pictures/wallpapers

# Use wipe transitions for hyprland
randpaper --renderer swww --transition-type wipe --transition-step 90 --transition-fps 60 ~/Pictures/wallpapers --backend hyprland
```

- Again, the above commands only work if there isn't already a `swww-daemon`
  instance running. Adding an `exec randpaper` to your config automatically adds
  an `exec swww-daemon`

- [Sample wallpaper repo](https://github.com/saylesss88/wallpapers2)

**Advanced Options**

| Flag                    | Description                              | Default       |
| :---------------------- | :--------------------------------------- | :------------ |
| `[DIR]`                 | Directory containing images              | `.`           |
| `-t, --time`            | Time between changes(e.g., 15m, 1h)      | `30m`         |
| `-b, --backend`         | Detection backend: `sway` or `hyprland`  | `sway`        |
| `-o, --outputs`         | Specific outputs to target (Sway only)   | Auto-discover |
| `-r, --renderer`        | Renderer tool: `swaybg` or `swww`        | `swaybg`      |
| `--transition-type`     | swww transition: `fade`, `wipe`, `outer` | `simple`      |
| `-s, --transition-step` | swww transition step (0-100)             | `90`          |
| `-f, --transition-fps`  | swww target frame rate for transitions   | `30`          |

- `--transition-type`: Choose between (`simple`, `fade`, `wipe`, `outer`,
  `inner`, `random`)

> NOTE: All transition options are ignored when using `--renderer swaybg`.

---

## 🎨 Automatic Terminal Theming

`randpaper` automatically extracts a 16-color palette from your wallpaper and
generates theme files in `~/.config/randpaper/themes/`.

To use them, add the include line to your terminal config:

**Ghostty**

File: `~/.config/ghostty/config`

```text
config-file = ~/.config/randpaper/themes/ghostty.config
```

**Foot**

File: `~/.config/foot/foot.ini` (Place near end of file)

```text
[main]
include=~/.config/randpaper/themes/foot.ini
```

**Kitty**

File: `~/.config/kitty/kitty.conf`

```text
allow_remote_control yes
listen_on unix:/tmp/mykitty
include ~/.config/randpaper/themes/kitty.conf
```

Add a keybind to your Sway or Hyprland config to start kitty with remote control
enabled (Sway example):

```conf
$mod+Shift+t exec kitty -o allow_remote_control=yes --listen-on unix:/tmp/mykitty
```

**Live-Reload of Terminal Themes**

- **Ghostty**: live reload works with either "cycle" keybind (i.e., theme
  updates immediately with either `pkill -USR1 randpaper` from either a keybind
  or the command line or a keybind set to
  `exec randpaper ~/Pictures/wallpapers`)(Tested on Fedora with Sway, expect
  different behavior on NixOS)

- **Kitty**: live reloads work with the one-shot keybind if you follow the steps
  above.

- **Foot**: the theme file is updated, but existing windows may not live-reload
  reliably; close and reopen the terminal to pick up the new theme. (Work in
  Progress)

<details>
<summary> ✔️GhosTTY/Kitty on NixOS dynamic theming </summary>

**GhosTTY**

```nix
# home.nix or ghostty.nix
programs.ghostty = {
  enable = true;
  settings = {
    # The '?' makes the include optional/non-blocking
    "config-file" = "?~/.config/randpaper/themes/ghostty.config";
  };
};
```

- On boot up, the generated theme will be applied.

- Run the cycle command then type `pkill -USR2 ghostty` to apply the theme
  instantly.

**Kitty**

```nix
programs.kitty = {
  extraConfig = ''
    allow_remote_control yes
    listen_on unix:/tmp/mykitty
    include ~/.config/randpaper/themes/kitty.conf
  '';
};
```

Add this keybind:

```nix
"$mod,T,exec,kitty -o allow_remote_control=yes --listen-on unix:/tmp/mykitty"
# One-Shot for Hyprland
"$mod SHIFT,N,exec, randpaper --backend hyprland /home/Your-User/Pictures/wallpapers"
```

Now running the above one-shot command will dynamically reload the kitty theme
and apply it automatically.

</details>

---

## 🫟 Waybar Dynamic Theming (Optional)

<details>
<summary> ✔️ Waybar Dynamic Theming </summary>

To use `randpaper` with a `waybar` setup, ensure you call the daemon via
`exec waybar` in your Sway or Hyprland config.

(Sway Example `~/.config/sway/config`):

```config
exec randpaper --time 10m ~/Pictures/wallpapers

bar {
    swaybar_command waybar
}
# ---snip---
```

`randpaper` automatically generates a Waybar CSS color file from the current
wallpaper palette and writes it to:

- `~/.config/randpaper/themes/waybar.css`

This file only defines color variables (it doesn't change your bar by itself).
To use it, you import it from your Waybar `style.css` and then reference the
variables in your existing rules.

Example of the generated `waybar.css` file:

```css
/* auto-generated by randpaper */
@define-color rp_bg #1c1c24;
@define-color rp_fg #e4dcd4;
@define-color rp_accent #f394a4;
@define-color rp_warn #201c24;
@define-color rp_ok #201c24;
@define-color rp_border #f394a4;
@define-color rp_muted #1c1c24;
```

1. Import the generated file

At the very top of your Waybar `style.css` (Change `YOUR_USER` to your
username):

```css
@import "/home/YOUR_USER/.config/randpaper/themes/waybar.css";
```

2. Use the variables in your CSS

Replace hard-coded colors with variables like `@rp_bg`, `@rp_fg`, `@rp_accent`,
etc.

Example:

```css
window#waybar {
  background-color: alpha(@rp_bg, 0.8);
  color: @rp_fg;
}

#workspaces button.focused {
  background-color: @rp_accent;
  color: @rp_bg;
}
```

- [Full `style.css` Example](https://github.com/saylesss88/.dotfiles/blob/main/waybar/.config/waybar/style.css)

3. `randpaper` automatically reloads Waybar when it changes wallpapers, keeping
   your bar perfectly themed without manual intervention. (`SIGUSR2`
   zero-flicker reload)

Obviously, the more hard-coded colors you replace with the dynamically generated
CSS variables the more noticeable it will be.

> NOTE: Just adding the `@import` at the top makes the variables available, you
> need to reference them for the changes to be applied.

- [NixOS Waybar Example](https://github.com/saylesss88/flake/blob/main/home/hypr/waybar.nix)

- On NixOS, after cycling with one-shot run `pkill -USR2 waybar` to apply the
  new theme.

</details>

---

## ⏭️ Cycling Wallpapers & Themes

### Manual Wallpaper Change (Recommended)

The simplest way to instantly change wallpapers is to run one-shot mode while
the randpaper daemon is running:

**Hyprland Config**:

```text
bind = $mainMod SHIFT, N, exec, randpaper ~/Pictures/wallpapers --backend hyprland
```

**Sway Config**:

```text
bindsym $mod+Shift+n exec randpaper ~/Pictures/wallpapers
```

This picks a new wallpaper, updates all themes (terminal + Waybar), and exits.
Your background daemon continues running for automatic cycles.

---

### Signal Running Daemon (Alternative for Standare Filesystem Hierarchy layouts)

You can also force the daemon to cycle immediately without spawning a separate
process:

**Sway Config**:

```text
bindsym $mod+n exec pkill -USR1 randpaper
```

**Hyprland Config**:

```text
bind = $mainMod SHIFT, N, exec, pkill -USR1 randpaper
```

**Shell**:

```bash
pkill -USR1 randpaper
```

Now when you run the above keybind, you will get a new wallpaper, terminal
theme, and waybar theme.

> NOTE: Expect different behavior on NixOS.

---

## ⚙️ How it Works

1. **Startup**: Caches all valid image paths (`jpg`, `png`, `bmp`, `webp`) from
   the target directory into memory.

2 **Loop**(daemon mode): Every interval (e.g., 30m):

- Queries active monitors via IPC.

- Picks a random image for the primary monitor and extracts its color palette.

- Generates theme files and triggers terminal reloads.

- Spawns a non-blocking background process (`swaybg` daemon or `swww` client) to
  update the display.

- Sleeps efficiently until the next cycle.

3. One-Shot (no `--time`): Picks wallpaper, updates themes, exits immediately.

- Works reliably as a keybind, may have to close out terminal and relaunch for
  the new theme to be applied.

---

## License

- [Apache License 2.0](https://github.com/saylesss88/randpaper/blob/main/LICENSE)
