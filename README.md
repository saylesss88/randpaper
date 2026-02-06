# randpaper - lightweight wallpaper utility

A minimalist, high-performance wallpaper daemon for Wayland. Randpaper keeps
your workspace fresh by assigning unique, randomized backgrounds to each of your
monitors at regular intervals.

## 🚀 Features

- Multi-Monitor Logic: Unlike basic scripts, Randpaper assigns a different
  random image to every active output simultaneously.

- Dual-Backend Support: Works out of the box with both swaybg (Sway/WLROOTS) and
  hyprpaper (Hyprland).

- Smart Defaults: Rotates your wallpapers every 30 minutes by default—just set
  the directory and forget it.

- Zero Bloat: Written in Rust with tokio for a near-zero memory footprint and
  asynchronous execution.

## 🛠 Installation

**Prerequisits**

You need one of the following backends installed depending on your compositor

- Sway: `swaybg`

- Hyprland: `hyprpaper`

```bash
cargo install randpaper
cargo install --git https://github.com/saylesss88/randpaper
```

## 🧾 Usage

```bash
# Defaults to changing every 30m
randpaper ~/Pictures/wallpapers
```

**Advanced Options**

| Flag            | Description                               | Default       |
| :-------------- | :---------------------------------------- | :------------ |
| `-t, --time`    | Time between changes(e.g., 15m, 1h)       | `30m`         |
| `-b, --backend` | Choose backend: `sway` or `hyprpaper`     | `sway`        |
| `-o, --outputs` | Specific outputs to target (e.g., `DP-1`) | Auto-discover |

Defaults to Sway, but also works with Hyprland:

```bash
randpaper --time=30m ~/Pictures/wallpapers --backend hyprpaper
```

## ⚙️ How it Works

- Randpaper operates as a lightweight daemon. Every interval, it:

- Scans your chosen directory for common image formats (`jpg`, `png`, `bmp`).

- Identifies all active monitors via IPC (Sway IPC or Hyprland API).

- Selects a unique random image for each monitor.

- Triggers the backend to update the display without flickering.
