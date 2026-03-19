# wl-harmonograph

Animated wallpaper for Sway/Wayland that draws
[harmonograph](https://en.wikipedia.org/wiki/Harmonograph) patterns in the
background. A harmonograph simulates the motion of a damped pendulum system -
two pendulums control the X axis and two control the Y axis, each with their
own frequency, phase, amplitude, and damping. The interference between these
pendulums traces out intricate, slowly decaying curves.

When a pattern finishes, the screen clears and a new one begins with fresh
random parameters, a different color, and potentially a different shape,
resulting in a unique wallpaper on every restart.

Eight shape types are available:

| Shape | Description |
|---|---|
| **harmonograph** | Four damped pendulums — the classic |
| **spirograph** | Hypotrochoid/epitrochoid curves (like the drawing toy) |
| **lissajous** | Two sinusoids with different frequencies |
| **rose** | Polar flower curves with optional compound petals |
| **butterfly** | Temple Fay's butterfly curve (1989) |
| **lorenz** | Lorenz strange attractor — chaotic double-spiral |
| **wireframe** | Rotating 3D Platonic solids (cube, icosahedron, …) |
| **torusknot** | 3D torus knots — curves wound around a torus |

<p align="center">
  <img src="https://github.com/user-attachments/assets/c6d704e0-d39b-4620-974b-209fdba3255a" width="30%" />
  <img src="https://github.com/user-attachments/assets/0a79ecc3-0fce-4a1c-9e63-c52ecebd2e21" width="30%" />
  <img src="https://github.com/user-attachments/assets/a0058140-5458-46e5-a5ae-8ae9ffd65737" width="30%" />
</p>

## Architecture

GPU-accelerated rendering using EGL + OpenGL ES 2.0 on top of
smithay-client-toolkit with wlr-layer-shell:

- Curve segments are rasterized on the GPU as triangle strips into an FBO
  that accumulates over time
- Each tick the CPU computes only 3 pendulum positions and submits 3
  triangle-strip draw calls
- Catmull-Rom spline interpolation for smooth anti-aliased curves
- Supports multiple monitors at native resolution
- Minimal CPU usage (~0.5% at 10fps)

## Usage

### Sway

Remove any existing `output * bg ...` or `exec swaybg` lines and add to your
Sway config (`~/.config/sway/config`):

```
exec wl-harmonograph
```

### Install

```bash
nix run github:pinpox/wl-harmonograph
```

Or:

```bash
nix profile install github:pinpox/wl-harmonograph
```

### Configuration

All settings are controlled with environment variables.

**Colors:**

```bash
# Foreground colors (comma-separated hex, cycles through them)
export HARMONOGRAPH_FG="#fb4934,#b8bb26,#fe8019"

# Background color
export HARMONOGRAPH_BG="#1d2021"
```

Default colors are gruvbox-inspired.

**Shape:**

| Variable | Default | Description |
|---|---|---|
| `HARMONOGRAPH_SHAPE` | `random` | Shape type or `random` to cycle through all |

**Rendering:**

| Variable | Default | Description |
|---|---|---|
| `HARMONOGRAPH_LINE_WIDTH` | `2.0` | Line thickness in pixels |
| `HARMONOGRAPH_ALPHA` | `0.85` | Line opacity (0.01–1.0) |
| `HARMONOGRAPH_FADE` | `0.005` | Trail fade speed per frame (0 = no fade) |
| `HARMONOGRAPH_SPEED` | `1` | Simulation steps per frame |
| `HARMONOGRAPH_FPS` | `30` | Target frame rate (1–144) |

**Dithering:**

8x8 ordered (Bayer) dithering applied to the final composite. Quantizes
colors to a fixed number of levels per channel with a spatial threshold
pattern that approximates the in-between shades. Low level counts produce a
retro, screen-printed look.

| Variable | Default | Description |
|---|---|---|
| `HARMONOGRAPH_DITHER` | `0.0` | Dithering strength (0 = off, 1 = full) |
| `HARMONOGRAPH_DITHER_LEVELS` | `8.0` | Color levels per channel (2 = 1-bit, 256 = subtle) |
| `HARMONOGRAPH_DITHER_SCALE` | `1.0` | Dither cell size in pixels (try 2–3 on HiDPI) |

Example — heavy retro dithering:

```bash
export HARMONOGRAPH_DITHER=1.0
export HARMONOGRAPH_DITHER_LEVELS=4
export HARMONOGRAPH_DITHER_SCALE=2
```

Example — subtle banding reduction:

```bash
export HARMONOGRAPH_DITHER=0.5
export HARMONOGRAPH_DITHER_LEVELS=32
```

### Live Control

While the wallpaper is running, you can tweak every parameter in real time
using the companion control tool:

```bash
wl-harmonograph-ctl
```

This opens an interactive TUI where you adjust line width, alpha, fade,
dithering, and all four pendulum parameters (frequency, amplitude, phase,
damping) with immediate visual feedback on the wallpaper.

```
● wl-harmonograph  shape: spirograph
  Drawing
  ▸ Line Width     ████████░░░░░░░░░░░░       2.0
    Alpha          █████████████████░░░       0.85
    Fade           █░░░░░░░░░░░░░░░░░░░     0.0050
    Speed          █░░░░░░░░░░░░░░░░░░░       1

  Spirograph
    Outer Radius   ██████████░░░░░░░░░░      1.000
    Inner Radius   ████░░░░░░░░░░░░░░░░      0.333
    ...

  ↑↓ select  ←→ adjust  shift+←→ fine  r random  s shape  c color  space restart  q quit
```

You can also send one-off commands from scripts:

```bash
wl-harmonograph-ctl get                   # dump all current values
wl-harmonograph-ctl set alpha 0.5         # set a parameter
wl-harmonograph-ctl set bg '#282828'      # change background color
wl-harmonograph-ctl set shape lorenz      # switch to a specific shape
wl-harmonograph-ctl randomize             # new random pattern + shape
wl-harmonograph-ctl next-color            # cycle foreground color
wl-harmonograph-ctl next-shape            # cycle to next shape type
wl-harmonograph-ctl restart               # clear canvas, redraw with current params
```

The daemon listens on `$XDG_RUNTIME_DIR/wl-harmonograph.sock`.

## Requirements

- A Wayland compositor supporting `wlr-layer-shell-unstable-v1` (Sway, Hyprland, etc.)
- OpenGL ES 2.0 capable GPU

## License

MIT

---

This project is a rewrite of my old [wallpaper-generator](https://github.com/pinpox/wallpaper-generator).
