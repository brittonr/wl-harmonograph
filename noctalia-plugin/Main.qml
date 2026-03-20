import QtQuick
import Quickshell
import Quickshell.Io
import qs.Commons

Item {
  id: root

  property var pluginApi: null

  property bool isRunning: false

  // Resolve settings with defaults
  readonly property bool autoStart: pluginApi?.pluginSettings?.autoStart ?? true
  readonly property string shape: pluginApi?.pluginSettings?.shape ?? "random"
  readonly property string fps: pluginApi?.pluginSettings?.fps ?? "30"
  readonly property int speed: pluginApi?.pluginSettings?.speed ?? 1
  readonly property real lineWidth: pluginApi?.pluginSettings?.lineWidth ?? 2.0
  readonly property real alpha: pluginApi?.pluginSettings?.alpha ?? 0.85
  readonly property real fade: pluginApi?.pluginSettings?.fade ?? 0.005
  readonly property real ditherStrength: pluginApi?.pluginSettings?.ditherStrength ?? 0.0
  readonly property real ditherLevels: pluginApi?.pluginSettings?.ditherLevels ?? 8
  readonly property real ditherScale: pluginApi?.pluginSettings?.ditherScale ?? 1.0
  readonly property bool useThemeColors: pluginApi?.pluginSettings?.useThemeColors ?? true
  readonly property string bgColor: pluginApi?.pluginSettings?.bgColor ?? "#1d1f23"
  readonly property string fgColors: pluginApi?.pluginSettings?.fgColors ?? "#fb4934,#98971a,#fcb157,#83a598,#d3869b,#8ec07c,#e4d398"

  // Build environment for wl-harmonograph
  function buildEnv() {
    let env = [];
    env.push("HARMONOGRAPH_SHAPE=" + shape);
    env.push("HARMONOGRAPH_FPS=" + fps);
    env.push("HARMONOGRAPH_SPEED=" + speed);
    env.push("HARMONOGRAPH_LINE_WIDTH=" + lineWidth);
    env.push("HARMONOGRAPH_ALPHA=" + alpha);
    env.push("HARMONOGRAPH_FADE=" + fade);
    env.push("HARMONOGRAPH_DITHER=" + ditherStrength);
    env.push("HARMONOGRAPH_DITHER_LEVELS=" + ditherLevels);
    env.push("HARMONOGRAPH_DITHER_SCALE=" + ditherScale);

    if (useThemeColors) {
      // Pull colors from the Noctalia theme
      let primary = Color.mPrimary.toString();
      let secondary = Color.mSecondary.toString();
      let tertiary = Color.mTertiary.toString();
      let bg = Color.mSurface.toString();
      env.push("HARMONOGRAPH_FG=" + primary + "," + secondary + "," + tertiary);
      env.push("HARMONOGRAPH_BG=" + bg);
    } else {
      env.push("HARMONOGRAPH_FG=" + fgColors);
      env.push("HARMONOGRAPH_BG=" + bgColor);
    }

    return env;
  }

  // Send a command to the running instance via wl-harmonograph-ctl
  function sendCtl(command) {
    ctlProcess.exec({
      "command": ["sh", "-c", "wl-harmonograph-ctl " + command]
    });
  }

  // Apply current settings to the running instance without restart
  function applyLive() {
    if (!isRunning) return;

    sendCtl("set line_width " + lineWidth);
    sendCtl("set alpha " + alpha);
    sendCtl("set fade " + fade);
    sendCtl("set speed " + speed);
    sendCtl("set dither " + ditherStrength);
    sendCtl("set dither_levels " + ditherLevels);
    sendCtl("set dither_scale " + ditherScale);

    if (useThemeColors) {
      let bg = Color.mSurface.toString();
      sendCtl("set bg " + bg.replace("#", ""));
    } else {
      sendCtl("set bg " + bgColor.replace("#", ""));
    }

    // Shape change requires a full restart of the pattern
    if (shape !== "random") {
      sendCtl("set shape " + shape);
    }
  }

  function startHarmonograph() {
    if (isRunning) return;

    // Kill any stale instance first
    Quickshell.execDetached(["sh", "-c", "pkill -f 'wl-harmonograph$' 2>/dev/null; sleep 0.2"]);

    let env = buildEnv();
    let envStr = env.join(" ");
    harmonographProcess.exec({
      "command": ["sh", "-c", "exec env " + envStr + " wl-harmonograph"]
    });

    // Check after a moment if the process stuck around
    startCheckTimer.running = true;
  }

  function stopHarmonograph() {
    Quickshell.execDetached(["sh", "-c", "pkill -f 'wl-harmonograph$' 2>/dev/null || true"]);
    isRunning = false;
  }

  function restartHarmonograph() {
    stopHarmonograph();
    restartTimer.running = true;
  }

  // IPC handler so other plugins/keybinds can control it
  IpcHandler {
    target: "plugin:wl-harmonograph"

    function start() { root.startHarmonograph(); }
    function stop() { root.stopHarmonograph(); }
    function restart() { root.restartHarmonograph(); }
    function randomize() { root.sendCtl("randomize"); }
    function nextShape() { root.sendCtl("next-shape"); }
    function nextColor() { root.sendCtl("next-color"); }
  }

  // The wl-harmonograph process
  Process {
    id: harmonographProcess
    stdout: StdioCollector {}
    stderr: StdioCollector {}
    onExited: function(exitCode) {
      root.isRunning = false;
      // Auto-restart on crash if autoStart is enabled
      if (root.autoStart && exitCode !== 0) {
        Logger.w("Harmonograph", "Process exited with code " + exitCode + ", restarting...");
        root.restartTimer.running = true;
      }
    }
  }

  // Process for sending ctl commands
  Process {
    id: ctlProcess
    stdout: StdioCollector {}
    stderr: StdioCollector {}
  }

  // Check if process started successfully
  Timer {
    id: startCheckTimer
    interval: 1500
    running: false
    repeat: false
    onTriggered: {
      if (harmonographProcess.running) {
        root.isRunning = true;
        Logger.i("Harmonograph", "Wallpaper started");
      }
    }
  }

  // Delay before restart
  Timer {
    id: restartTimer
    interval: 500
    running: false
    repeat: false
    onTriggered: root.startHarmonograph()
  }

  // Auto-start on plugin load
  Component.onCompleted: {
    if (autoStart) {
      startHarmonograph();
    }
  }

  // Clean up on unload
  Component.onDestruction: {
    stopHarmonograph();
  }

  // Re-apply colors when theme changes
  Connections {
    target: Color
    function onMPrimaryChanged() {
      if (root.useThemeColors && root.isRunning) {
        root.applyLive();
      }
    }
  }
}
