import QtQuick
import QtQuick.Layouts
import Quickshell
import qs.Commons
import qs.Widgets

ColumnLayout {
  id: root
  spacing: Style.marginL

  property var pluginApi: null

  // Local editing state, initialized from saved settings
  property bool editAutoStart: pluginApi?.pluginSettings?.autoStart ?? true
  property string editShape: pluginApi?.pluginSettings?.shape ?? "random"
  property string editFps: pluginApi?.pluginSettings?.fps ?? "30"
  property int editSpeed: pluginApi?.pluginSettings?.speed ?? 1
  property real editLineWidth: pluginApi?.pluginSettings?.lineWidth ?? 2.0
  property real editAlpha: pluginApi?.pluginSettings?.alpha ?? 0.85
  property real editFade: pluginApi?.pluginSettings?.fade ?? 0.005
  property real editDitherStrength: pluginApi?.pluginSettings?.ditherStrength ?? 0.0
  property real editDitherLevels: pluginApi?.pluginSettings?.ditherLevels ?? 8
  property real editDitherScale: pluginApi?.pluginSettings?.ditherScale ?? 1.0
  property bool editUseThemeColors: pluginApi?.pluginSettings?.useThemeColors ?? true
  property string editBgColor: pluginApi?.pluginSettings?.bgColor ?? "#1d1f23"
  property string editFgColors: pluginApi?.pluginSettings?.fgColors ?? "#fb4934,#98971a,#fcb157,#83a598,#d3869b,#8ec07c,#e4d398"

  function saveSettings() {
    if (!pluginApi) return;

    pluginApi.pluginSettings.autoStart = root.editAutoStart;
    pluginApi.pluginSettings.shape = root.editShape;
    pluginApi.pluginSettings.fps = root.editFps;
    pluginApi.pluginSettings.speed = root.editSpeed;
    pluginApi.pluginSettings.lineWidth = root.editLineWidth;
    pluginApi.pluginSettings.alpha = root.editAlpha;
    pluginApi.pluginSettings.fade = root.editFade;
    pluginApi.pluginSettings.ditherStrength = root.editDitherStrength;
    pluginApi.pluginSettings.ditherLevels = root.editDitherLevels;
    pluginApi.pluginSettings.ditherScale = root.editDitherScale;
    pluginApi.pluginSettings.useThemeColors = root.editUseThemeColors;
    pluginApi.pluginSettings.bgColor = root.editBgColor;
    pluginApi.pluginSettings.fgColors = root.editFgColors;

    pluginApi.saveSettings();
  }

  // ── General ──

  NHeader {
    label: "Harmonograph Wallpaper"
    description: "Animated mathematical curves rendered as a Wayland wallpaper"
  }

  NToggle {
    label: "Auto Start"
    description: "Launch the wallpaper when Noctalia starts"
    checked: root.editAutoStart
    onToggled: checked => root.editAutoStart = checked
  }

  // ── Shape ──

  NDivider { Layout.fillWidth: true }

  NComboBox {
    Layout.fillWidth: true
    label: "Shape"
    description: "Mathematical curve to draw (random picks a new one each cycle)"
    model: [
      { "key": "random",        "name": "Random" },
      { "key": "harmonograph",  "name": "Harmonograph" },
      { "key": "spirograph",    "name": "Spirograph" },
      { "key": "lissajous",     "name": "Lissajous" },
      { "key": "rose",          "name": "Rose" },
      { "key": "butterfly",     "name": "Butterfly" },
      { "key": "lorenz",        "name": "Lorenz Attractor" },
      { "key": "rossler",       "name": "Rössler Attractor" },
      { "key": "clifford",      "name": "Clifford Attractor" },
      { "key": "dejong",        "name": "De Jong Attractor" },
      { "key": "superformula",  "name": "Superformula" },
      { "key": "guilloche",     "name": "Guilloché" },
      { "key": "dopendulum",    "name": "Double Pendulum" },
      { "key": "wireframe",     "name": "Wireframe Polyhedra" },
      { "key": "torusknot",     "name": "Torus Knot" },
      { "key": "surface",       "name": "3D Surface" }
    ]
    currentKey: root.editShape
    onSelected: key => root.editShape = key
  }

  // ── Drawing ──

  NDivider { Layout.fillWidth: true }

  NHeader {
    label: "Drawing"
  }

  NComboBox {
    Layout.fillWidth: true
    label: "Frame Rate"
    model: [
      { "key": "30",  "name": "30 FPS" },
      { "key": "60",  "name": "60 FPS" },
      { "key": "120", "name": "120 FPS" }
    ]
    currentKey: root.editFps
    onSelected: key => root.editFps = key
  }

  NValueSlider {
    Layout.fillWidth: true
    label: "Speed"
    description: "Steps per frame — higher fills the pattern faster"
    value: root.editSpeed
    from: 1
    to: 20
    stepSize: 1
    onMoved: value => root.editSpeed = Math.round(value)
  }

  NValueSlider {
    Layout.fillWidth: true
    label: "Line Width"
    value: root.editLineWidth
    from: 0.5
    to: 10.0
    stepSize: 0.5
    onMoved: value => root.editLineWidth = value
  }

  NValueSlider {
    Layout.fillWidth: true
    label: "Opacity"
    description: "Line alpha — lower values produce more translucent strokes"
    value: root.editAlpha
    from: 0.05
    to: 1.0
    stepSize: 0.05
    onMoved: value => root.editAlpha = value
  }

  NValueSlider {
    Layout.fillWidth: true
    label: "Fade"
    description: "How fast old trails disappear"
    value: root.editFade
    from: 0.0
    to: 0.05
    stepSize: 0.001
    onMoved: value => root.editFade = value
  }

  // ── Dithering ──

  NDivider { Layout.fillWidth: true }

  NHeader {
    label: "Dithering"
  }

  NValueSlider {
    Layout.fillWidth: true
    label: "Strength"
    description: "0 = off, higher adds ordered dithering for a retro look"
    value: root.editDitherStrength
    from: 0.0
    to: 1.0
    stepSize: 0.05
    onMoved: value => root.editDitherStrength = value
  }

  NValueSlider {
    Layout.fillWidth: true
    visible: root.editDitherStrength > 0
    label: "Levels"
    value: root.editDitherLevels
    from: 2
    to: 64
    stepSize: 2
    onMoved: value => root.editDitherLevels = Math.round(value)
  }

  NValueSlider {
    Layout.fillWidth: true
    visible: root.editDitherStrength > 0
    label: "Scale"
    value: root.editDitherScale
    from: 1.0
    to: 8.0
    stepSize: 0.5
    onMoved: value => root.editDitherScale = value
  }

  // ── Colors ──

  NDivider { Layout.fillWidth: true }

  NHeader {
    label: "Colors"
  }

  NToggle {
    label: "Use Theme Colors"
    description: "Pull foreground and background from the Noctalia color scheme"
    checked: root.editUseThemeColors
    onToggled: checked => root.editUseThemeColors = checked
  }

  NTextInput {
    Layout.fillWidth: true
    visible: !root.editUseThemeColors
    label: "Background Color"
    description: "Hex color for the wallpaper background"
    placeholderText: "#1d1f23"
    text: root.editBgColor
    onTextChanged: root.editBgColor = text
  }

  NTextInput {
    Layout.fillWidth: true
    visible: !root.editUseThemeColors
    label: "Foreground Colors"
    description: "Comma-separated hex colors for the curves"
    placeholderText: "#fb4934,#98971a,#fcb157"
    text: root.editFgColors
    onTextChanged: root.editFgColors = text
  }

  // ── Actions ──

  NDivider { Layout.fillWidth: true }

  NHeader {
    label: "Quick Actions"
    description: "These take effect immediately on the running wallpaper"
  }

  RowLayout {
    spacing: Style.marginM

    NButton {
      text: "Randomize"
      outlined: true
      onClicked: Quickshell.execDetached(["wl-harmonograph-ctl", "randomize"])
    }

    NButton {
      text: "Next Shape"
      outlined: true
      onClicked: Quickshell.execDetached(["wl-harmonograph-ctl", "next-shape"])
    }

    NButton {
      text: "Next Color"
      outlined: true
      onClicked: Quickshell.execDetached(["wl-harmonograph-ctl", "next-color"])
    }

    NButton {
      text: "Clear"
      outlined: true
      onClicked: Quickshell.execDetached(["wl-harmonograph-ctl", "restart"])
    }
  }
}
