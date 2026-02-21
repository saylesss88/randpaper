[![Nix Flake](https://img.shields.io/badge/Nix_Flake-Geared-dddd00?logo=nixos&logoColor=white)](https://nixos.org/manual/nix/stable/command-ref/new-cli/nix3-flake.html)

[![Nix](https://img.shields.io/badge/Nix-5277C3?style=flat&logo=nixos&logoColor=white)](https://nixos.org)

# randpaper

![randpaper demo](https://raw.githubusercontent.com/saylesss88/randpaper/main/assets/demo.gif)

Fast per-monitor wallpaper rotation + optional theme syncing for Wayland
(Sway/Hyprland).

`randpaper` caches your wallpaper list once, picks random images per output, and
can optionally generate theme files (terminals + Waybar) from the current
wallpaper.

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
  Sway IPC or `hyprctl`) and supports both `swaybg` and `awww` renderers.

- Support for both `awww` and `swww` for compatibility.

---

## 🛠 Installation

**Prerequisites**

You need `swaybg` or (`awww` / `swww`) installed as the renderer.

```bash
# From source
cargo install --git https://github.com/saylesss88/randpaper
# crates.io
cargo install randpaper
```

After `randpaper` and `awww` are installed, you can ensure that it automatically
detects your monitors and see which transition you like with commands like:

```bash
# Use fade transitions
randpaper --renderer awww --transition-type fade --transition-step 90 --transition-fps 60 --wallpaper-dir ~/Pictures/wallpapers

# Use wipe transitions for hyprland
randpaper --renderer awww --transition-type wipe --transition-step 90 --transition-fps 60 --wallpaper-dir ~/Pictures/wallpapers --backend hyprland
```

Once you find what you like, either add an `exec` to the chosen command, or
throw the options in a `config.toml` and simplify the `exec` greatly.

---

## 📚️ Configuration (Optional)

Config file (XDG):

- `~/.config/randpaper/config.toml`

Example:

```toml
wallpaper_dir = "/home/user/Pictures/wallpapers"
backend = "sway"        # "sway" | "hyprland"
renderer = "awww"       # "swaybg" | "awww"
time = "30m"            # used only in daemon mode

transition_type = "wipe"
transition_step = 90
transition_fps = 60

# outputs = ["DP-1", "HDMI-A-1"]
```

Precidence:

- CLI args override `config.toml`.

---

## Usage

**Command flag table**

| Flag                    | Description                                  | Default       |
| :---------------------- | :------------------------------------------- | :------------ |
| `[DIR]`                 | Directory containing images                  | `.`           |
| `-t, --time`            | Time between changes(e.g., 15m, 1h)          | NA            |
| `--daemon`              | Activate daemon-mode, `--time` also required | NA            |
| `-w, --wallpaper-dir`   | Directory containing wallpaper images        | `.`           |
| `--config`              | Directory containing `config.toml`           | NA            |
| `-b, --backend`         | Detection backend: `sway` or `hyprland`      | `sway`        |
| `-o, --outputs`         | Specific outputs to target (Sway only)       | Auto-discover |
| `-r, --renderer`        | Renderer tool: `swaybg` or `awww`            | `swaybg`      |
| `--transition-type`     | swww transition: `fade`, `wipe`, `outer`     | `simple`      |
| `-s, --transition-step` | swww transition step (0-100)                 | `90`          |
| `-f, --transition-fps`  | swww target frame rate for transitions       | `30`          |

- `--transition-type`: Choose between (`simple`, `fade`, `wipe`, `outer`,
  `inner`, `random`)

> NOTE: All transition options are ignored when using `--renderer swaybg`.

---

### One-shot (default)

Applies a wallpaper + updates themes once, then exits.

```sh
# Runs with settings from `config.toml`
randpaper
# Without using the `config.toml` any command without both `--daemon` & `--time` set
# are one-shot commands
randpaper --backend hyprland --renderer swaybg --wallpaper-dir ~/Pictures/wallpapers
# or
randpaper --daemon -t 1m -b sway -r awww --transition-type wipe
```

Recommended one-shot keybinds:

**Sway**

```text
bindsym $mod+Shift+n exec randpaper
```

**Hyprland**

```text
bind = $mainMod SHIFT, N, exec, randpaper
```

The above keybinds cycle wallpapers & themes without spawning a `--daemon`
process. This is also the recommended way to cycle wallpapers & themes **after**
the daemon is started. (This prevents multiple `--daemon` processes being
spawned)

---

### Daemon mode (`--daemon`)

Runs continuously and rotates wallpapers on a timer.

**Daemon mode Requirements**:

- `--daemon` must be provided.

- `time` must be set (via `config.toml` or `--time`).

Recommended autostart:

**Sway**

```text
# With `config.toml`
exec randpaper --daemon
# Or something like this without using the config:
exec randpaper -w ~/Pictures/wallpapers -t 5m -r awww --daemon
```

**Hyprland**

```text
exec-once = randpaper --daemon
```

On standard filesystem hierarchy systems you can also force the daemon to cycle
without spawning a separate process:

```bash
pkill -USR1 randpaper
```

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

---

## 🫟 Waybar Dynamic Theming

<details>
<summary> ✔️ Waybar Dynamic Theming </summary>

To use `randpaper` with a `waybar` setup, ensure you call the daemon via
`exec waybar` in your Sway or Hyprland config.

(Sway Example `~/.config/sway/config`):

```config
exec randpaper --time 15m --wallpaper-dir ~/Pictures/wallpapers --daemon

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
  background-color: @rp_bg;
  color: @rp_accent;
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

- On NixOS, after cycling with one-shot run `pkill -USR2 waybar` to apply the
  new theme.

</details>

## NixOS

<details>
<summary> ✔️ NixOS </summary>

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

### Using the `config.toml` on NixOS

To use the config file on NixOS you can do something like this:

```nix
home.file = {
  ".config/randpaper/config.toml".text = ''
      # ~/.config/randpaper/config.toml
      time = "10m"
      renderer = "awww"
      backend = "hyprland"
      wallpaper_dir = "/home/user/Pictures/Wallpapers"
  '';
};
```

And now your `exec` can be simplified to:

```nix
exec-once = [
  "randpaper --daemon"
];
```

### Terminal & Waybar Theming on NixOS

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

**Kitty (Recommended on NixOS)**

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

### Waybar on NixOS

- [NixOS Waybar Example](https://github.com/saylesss88/flake/blob/main/home/hypr/waybar.nix)

</details>

## ⚙️ How it Works

1. **Startup**: Caches all valid image paths (`jpg`, `png`, `bmp`, `webp`) from
   the target directory into memory.

2 **Loop**(daemon mode): Every interval (e.g., 30m):

- Queries active monitors via IPC.

- Picks a random image for the primary monitor and extracts its color palette.

- Generates theme files and triggers terminal reloads.

- Spawns a non-blocking background process (`swaybg` daemon or `awww` client) to
  update the display.

- Sleeps efficiently until the next cycle.

3. One-Shot (no `--time` or `--daemon`): Picks wallpaper, updates themes, exits
   immediately.

---

## License

- [Apache License 2.0](https://github.com/saylesss88/randpaper/blob/main/LICENSE)
