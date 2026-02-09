# randpaper - lightweight wallpaper & theme utility

A minimalist, high-performance wallpaper daemon for Wayland. `randpaper` keeps
your workspace fresh by assigning unique, randomized backgrounds to each of your
monitors at regular intervals and optionally syncing your terminal theme to
match the image pallete.

## 🚀 Features

- ⚡ Performance: Written in Rust using tokio. Scans your image directory once
  into memory (caching paths) to prevent disk I/O spikes, even with massive
  wallpaper collections.

- 🎨 Dynamic Theming: Automatically extracts dominant colors from the current
  wallpaper and generates config files for Ghostty, Kitty, and Foot, keeping
  your terminal in sync with your desktop.

- 🖥️ Multi-Monitor: Assigns a unique random image to every active output
  simultaneously.

- 🛠️ Modular Backends: Works seamlessly with swaybg

---

## 🛠 Installation

**Prerequisits**

You need `swaybg` installed as the backend.

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

Hyprland UWSM:

```nix
wayland.windowManager.hyprland = {
  settings = {
    exec-once = [
  "uwsm app -- randpaper --time=15m /home/your-user/Pictures/wallpapers --backend hyprland"
    ];
  };
}
```

- It's not required to also ad an exec for `swaybg`.

Sway Example:

```nix
wayland.windowManager.sway = {
  extraConfig = ''
    exec randpaper --time 15m /home/your-user/wallpapers
    # Or use the defaults of `--backend sway` and `--time 30m`
    # exec randpaper /home/your-user/wallpapers
  '';
};
```

---

## 🧾 Usage

```bash
# Defaults to changing every 30m
randpaper ~/Pictures/wallpapers
# Custom: change every 5 minutes using Hyprland backend
randpaper --time 5m --backend hyprland ~/Pictures/wallpapers
```

**Advanced Options**

| Flag            | Description                               | Default       |
| :-------------- | :---------------------------------------- | :------------ |
| `-t, --time`    | Time between changes(e.g., 15m, 1h)       | `30m`         |
| `-b, --backend` | Choose backend: `sway` or `hyprland`      | `sway`        |
| `-o, --outputs` | Specific outputs to target (e.g., `DP-1`) | Auto-discover |

---

## 🎨 Automatic Terminal Theming

randpaper automatically extracts a 16-color palette from your wallpaper and
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

---

## ⚙️ How it Works

1. **Startup**: Caches all valid image paths (`jpg`, `png`, `bmp`, `webp`) from
   the target directory into memory.

2 **Loop**: Every interval (e.g., 30m):

- Queries active monitors via IPC.

- Picks a random image for the primary monitor and extracts its color palette.

- Generates theme files and triggers terminal reloads.

- Spawns a non-blocking background process (`swaybg`) to render the wallpapers.

- Sleeps efficiently until the next cycle.
