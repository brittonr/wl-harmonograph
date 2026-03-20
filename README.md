# wl-walls

Animated wallpaper for Sway/Wayland that draws mathematical curves in the
background. Shapes range from classic
[harmonograph](https://en.wikipedia.org/wiki/Harmonograph) pendulums to
chaotic attractors and 3D surface projections. Each curve traces out slowly,
accumulating trails that fade over time.

When a pattern finishes, the screen clears and a new one begins with fresh
random parameters, a different color, and potentially a different shape,
resulting in a unique wallpaper on every restart.

Fifteen shape types are available:

| Shape | Description |
|---|---|
| **harmonograph** | Four damped pendulums — the classic |
| **spirograph** | Hypotrochoid/epitrochoid curves (like the drawing toy) |
| **lissajous** | Two sinusoids with different frequencies |
| **rose** | Polar flower curves with optional compound petals |
| **butterfly** | Temple Fay's butterfly curve (1989) |
| **lorenz** | Lorenz strange attractor — chaotic double-spiral |
| **rossler** | Rössler attractor — folded-band spiral |
| **clifford** | Clifford attractor — feathery swirl patterns |
| **dejong** | De Jong attractor — symmetric star/floral structures |
| **superformula** | Gielis superformula — starfish to polygons |
| **guilloche** | Banknote-style engraving patterns |
| **dopendulum** | Double pendulum — chaotic arm motion |
| **wireframe** | Rotating 3D Platonic solids (cube, icosahedron, …) |
| **torusknot** | 3D torus knots — curves wound around a torus |
| **surface** | 3D surfaces (torus, sphere, Möbius strip, …) |

<p align="center">
  <img src="https://github.com/user-attachments/assets/c6d704e0-d39b-4620-974b-209fdba3255a" width="30%" />
  <img src="https://github.com/user-attachments/assets/0a79ecc3-0fce-4a1c-9e63-c52ecebd2e21" width="30%" />
  <img src="https://github.com/user-attachments/assets/a0058140-5458-46e5-a5ae-8ae9ffd65737" width="30%" />
</p>

## ASCII Mode

A standalone terminal renderer draws the same patterns as density-mapped
ASCII art — no Wayland or GPU required. Each cell accumulates intensity as
curves pass through it and maps to a character ramp (` .,:;=!*#$@`).

```bash
wl-walls --ascii                          # random shape
WALLS_SHAPE=wireframe wl-walls --ascii    # specific shape
```

Keys: `r` randomize, `s` next shape, `c` next color, `space` restart, `q` quit.

Reads the same `WALLS_*` env vars. Speed defaults to 50 (steps per
frame) since each step plots a single point rather than a GPU-interpolated
segment.

## Architecture

GPU-accelerated rendering using EGL + OpenGL ES 2.0 on top of
smithay-client-toolkit with wlr-layer-shell:

- Curve segments are rasterized on the GPU as triangle strips into an FBO
  that accumulates over time
- Each tick the CPU computes only a few curve positions and submits
  triangle-strip draw calls
- Catmull-Rom spline interpolation for smooth anti-aliased curves
- Supports multiple monitors at native resolution
- Minimal CPU usage (~0.5% at 10fps)

## Usage

### Sway

Remove any existing `output * bg ...` or `exec swaybg` lines and add to your
Sway config (`~/.config/sway/config`):

```
exec wl-walls
```

### Install

```bash
nix run github:brittonr/wl-walls
```

Or:

```bash
nix profile install github:brittonr/wl-walls
```

### Configuration

All settings are controlled with environment variables.

**Colors:**

```bash
# Foreground colors (comma-separated hex, cycles through them)
export WALLS_FG="#fb4934,#b8bb26,#fe8019"

# Background color
export WALLS_BG="#1d2021"
```

Default colors are gruvbox-inspired.

**Shape:**

| Variable | Default | Description |
|---|---|---|
| `WALLS_SHAPE` | `random` | Shape type or `random` to cycle through all |

**Rendering:**

| Variable | Default | Description |
|---|---|---|
| `WALLS_LINE_WIDTH` | `2.0` | Line thickness in pixels |
| `WALLS_ALPHA` | `0.85` | Line opacity (0.01–1.0) |
| `WALLS_FADE` | `0.005` | Trail fade speed per frame (0 = no fade) |
| `WALLS_SPEED` | `1` | Simulation steps per frame |
| `WALLS_FPS` | `30` | Target frame rate (1–144) |

**Dithering:**

8x8 ordered (Bayer) dithering applied to the final composite. Quantizes
colors to a fixed number of levels per channel with a spatial threshold
pattern that approximates the in-between shades. Low level counts produce a
retro, screen-printed look.

| Variable | Default | Description |
|---|---|---|
| `WALLS_DITHER` | `0.0` | Dithering strength (0 = off, 1 = full) |
| `WALLS_DITHER_LEVELS` | `8.0` | Color levels per channel (2 = 1-bit, 256 = subtle) |
| `WALLS_DITHER_SCALE` | `1.0` | Dither cell size in pixels (try 2–3 on HiDPI) |

Example — heavy retro dithering:

```bash
export WALLS_DITHER=1.0
export WALLS_DITHER_LEVELS=4
export WALLS_DITHER_SCALE=2
```

Example — subtle banding reduction:

```bash
export WALLS_DITHER=0.5
export WALLS_DITHER_LEVELS=32
```

### Live Control

While the wallpaper is running, you can tweak every parameter in real time
using the companion control tool:

```bash
wl-walls-ctl
```

This opens an interactive TUI where you adjust line width, alpha, fade,
dithering, and all shape parameters with immediate visual feedback on the
wallpaper.

```
● wl-walls  shape: spirograph
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
wl-walls-ctl get                   # dump all current values
wl-walls-ctl set alpha 0.5         # set a parameter
wl-walls-ctl set bg '#282828'      # change background color
wl-walls-ctl set shape lorenz      # switch to a specific shape
wl-walls-ctl randomize             # new random pattern + shape
wl-walls-ctl next-color            # cycle foreground color
wl-walls-ctl next-shape            # cycle to next shape type
wl-walls-ctl restart               # clear canvas, redraw with current params
```

The daemon listens on `$XDG_RUNTIME_DIR/wl-walls.sock`.

## Requirements

- A Wayland compositor supporting `wlr-layer-shell-unstable-v1` (Sway, Hyprland, etc.)
- OpenGL ES 2.0 capable GPU

## License

MIT
