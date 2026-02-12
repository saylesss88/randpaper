[![Nix Flake](https://img.shields.io/badge/Nix_Flake-Geared-dddd00?logo=nixos&logoColor=white)](https://nixos.org/manual/nix/stable/command-ref/new-cli/nix3-flake.html)

[![Nix](https://img.shields.io/badge/Nix-5277C3?style=flat&logo=nixos&logoColor=white)](https://nixos.org)

# randpaper - lightweight wallpaper & theme utility

A minimalist, high-performance wallpaper daemon for Wayland. `randpaper` keeps
your workspace fresh by assigning unique, randomized backgrounds to each of your
monitors at regular intervals and optionally syncing your terminal theme to
match the image palette.

## 🚀 Features

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

---

## 🛠 Installation

**Prerequisits**

You need `swaybg` or (`swww` / `awww`) installed as the renderer.

```bash
# From source
cargo install --git https://github.com/saylesss88/randpaper
# crates.io
cargo install randpaper
```

**Nix**

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

And add an `exec` for either hyprland or sway:

Hyprland Example:

```nix
wayland.windowManager.hyprland = {
  settings = {
    exec-once = [
  # Standard Usage
  "randpaper --time 15m /home/your-user/Pictures/wallpapers --backend hyprland --renderer swww"
  # UWSM Usage
  "uwsm app -- randpaper --time 15m /home/your-user/Pictures/wallpapers --backend hyprland"
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

> Note: `randpaper` manages the renderer process for you. You do not need a
> separate `exec-once = swaybg ...` or `exec-once = swww` line in your config.

---

## 🧾 Usage

**Daemon Mode (Background Process)**

When you provide the `--time` flag, `randpaper` runs as a daemon and
automatically cycles wallpapers at the specified interval:

```bash
# Change every 30 minutes (default time if --time not specified with daemon)
randpaper --time 30m ~/Pictures/wallpapers

# Change every 5 minutes using Hyprland + swww transitions
randpaper --time 5m --backend hyprland --renderer swww ~/Pictures/wallpapers

# Change every hour with custom transitions
randpaper --time 1h --renderer swww --transition-type fade ~/Pictures/wallpapers
```

**One-Shot Mode (Pick Once & Exit)**

Without the `--time` flag, `randpaper` picks a random wallpaper, updates themes,
and exits immediately:

```bash
# Pick one wallpaper + update themes, then exit
randpaper ~/Pictures/wallpapers

# Same, but with Hyprland backend
randpaper --backend hyprland ~/Pictures/wallpapers
```

**Use cases for one-shot mode**:

- Manual wallpaper changes via keybinds

- Scripted theme updates

- Testing without running a daemon

**swww/awww Transitions**:

```bash
# Use fade transitions
randpaper --renderer swww --transition-type fade --transition-step 90 --transition-fps 60 ~/Pictures/wallpapers

# Use wipe transitions
randpaper --renderer swww --transition-type wipe --transition-step 90 --transition-fps 60 ~/Pictures/wallpapers
```

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
include ~/.config/randpaper/themes/kitty.conf
```

> Note: `randpaper` attempts to live-reload `foot` (via `SIGUSR1`) and `kitty`
> (via `kitten`) automatically when the wallpaper changes.

- You may have to close the terminal and reopen it for the new theme to be
  applied.

---

## 🫟 Waybar Dynamic Theming (optional)

<details>
<summary> ✔️ Waybar Dynamic Theming </summary>

> ⚠️ Important: Avoid Duplicate Waybar Instances If you are using `randpaper`
> for dynamic theming, it will automatically launch `waybar` for you. Do not add
> exec `waybar` to your WM/compositor config, as this will cause multiple
> instances to run simultaneously.

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

3. `randpaper` automatically reloads Waybar when it changes wallpapers, keeping
   your bar perfectly themed without manual intervention. (`SIGUSR2`
   zero-flicker reload)

Obviously, the more hard-coded colors you replace with the dynamically generated
CSS variables the more noticeable it will be.

> NOTE: Just adding the `@import` at the top makes the variables available, you
> need to reference them for the changes to be applied.

</details>

---

## ⏭️ Cycling Wallpapers & Themes

### Manual Wallpaper Change (Recommended)

The simplest way to instantly change wallpapers is to run one-shot mode while
your daemon is running:

**Hyprland Config**:

```text
bind = $mainMod SHIFT, N, exec, randpaper ~/Pictures/wallpapers
```

**Sway Config**:

```text
bindsym $mod+Shift+n exec randpaper ~/Pictures/wallpapers
```

This picks a new wallpaper, updates all themes (terminal + Waybar), and exits.
Your background daemon continues running for automatic cycles.

---

### Signal Running Daemon (Alternative)

You can also force the daemon to cycle immediately without spawning a separate
process:

**Hyprland Config**:

```text
bind = $mainMod, N, exec, pkill -USR1 randpaper
```

**Sway Config**:

```text
bindsym $mod+n exec pkill -USR1 randpaper
```

**Shell**:

```bash
pkill -USR1 randpaper
```

Now when you run the above keybind, you will get a new wallpaper, terminal
theme, and waybar theme.

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

---

## License

- [Apache License 2.0](https://github.com/saylesss88/randpaper/blob/main/LICENSE)
