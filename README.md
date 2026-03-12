# wl-harmonograph

Animated wallpaper for Sway/Wayland that draws
[harmonograph](https://en.wikipedia.org/wiki/Harmonograph) patterns in the
background. A harmonograph simulates the motion of a damped pendulum system -
two pendulums control the X axis and two control the Y axis, each with their
own frequency, phase, amplitude, and damping. The interference between these
pendulums traces out intricate, slowly decaying curves.

When a pattern finishes, the screen clears and a new one begins with fresh
random parameters and a different color, resulting in a unique wallpaper on
every restart.

<p align="center">
  <img src="https://github.com/user-attachments/assets/c6d704e0-d39b-4620-974b-209fdba3255a" width="30%" />
  <img src="https://github.com/user-attachments/assets/0a79ecc3-0fce-4a1c-9e63-c52ecebd2e21" width="30%" />
  <img src="https://github.com/user-attachments/assets/a0058140-5458-46e5-a5ae-8ae9ffd65737" width="30%" />
</p>

## Usage

### Sway

Remove any existing `output * bg ...` or `exec swaybg` lines and add to your
Sway config (`~/.config/sway/config`):

```
exec wl-harmonograph
```

Colors can optionally be customized with environment variables;

```
exec HARMONOGRAPH_FG=ebdbb2,fb4934,b8bb26 HARMONOGRAPH_BG=282828 wl-harmonograph
```

The app creates a [gtk-layer-shell](https://github.com/wmww/gtk-layer-shell)
window pinned to the background layer on each monitor. A single offscreen
Cairo surface is rendered at the largest monitor's resolution and shared across
all screens. Each frame advances the pendulum simulation by one step and draws
a single Catmull-Rom interpolated curve segment to minimize CPU impact.

This project is a rewrite of my old [wallpaper-generator](https://github.com/pinpox/wallpaper-generator).
