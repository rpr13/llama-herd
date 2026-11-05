# Hybrid Theme System

LlamaHerd uses a Hybrid Theme System that combines a **Functional Palette** with **Procedural UI** behaviors and a rich suite of **popular built-in programmer themes**. This allows for a flexible, aesthetic TUI that adapts seamlessly to diverse terminal environments and user preferences.

---

## 🎨 Built-in Theme Presets

LlamaHerd ships with 8 truly free, permissively-licensed (MIT / Apache 2.0) built-in theme presets, plus support for a fully custom `theme.toml`:

| Preset Name | Config ID | License | Description |
| :--- | :--- | :--- | :--- |
| **Default** | `default` | MIT | Borderless Simple clean terminal styling with native transparent background. |
| **Gruvbox Dark** | `gruvbox-dark` | MIT | Community Retro Groove aesthetic with warm, earthy tones and `#282828` background. |
| **VS Code Dark+** | `vscode-dark` | MIT | Visual Studio Code modern dark palette with `#1e1e1e` background and blue accents. |
| **IntelliJ Dark** | `intellij-dark` | Apache 2.0 | JetBrains Darcula IDE dark theme with `#2b2b2b` background and teal/purple accents. |
| **Catppuccin Mocha** | `catppuccin-mocha` | MIT | Soothing pastel dark palette with `#1e1e2e` background and lavender accents. |
| **Tokyo Night** | `tokyo-night` | MIT | Clean dark aesthetic celebrating Tokyo night lights with `#1a1b26` background. |
| **Nord** | `nord` | MIT | Arctic, north-bluish clean palette with `#2e3440` background and frost accents. |
| **One Dark** | `one-dark` | MIT | Iconic Atom One Dark palette with `#282c34` background and vibrant accents. |
| **Custom** | `custom` | Permissive | Fully customizable palette loaded from your local `theme.toml` file. |

You can switch themes interactively in **Tab 2 (Settings)** by highlighting **Theme Preset** and pressing `Enter`, or by setting `theme = "<id>"` in `config.toml`.

---

## 🪟 Independent Transparent Background (`theme-transparent`)

Every theme preset can be enjoyed in two modes:
1. **Authentic Solid Background (Default for presets)**: Renders the exact background color of the theme (e.g. `#1e1e1e` for VS Code, `#282828` for Gruvbox).
2. **Transparent Background (`theme-transparent = true`)**: Sets `bg`, `header-bg`, and `footer-bg` to `Color::Reset`, allowing your terminal emulator's native background, wallpaper, or blur/transparency to show through, while preserving all syntax and UI highlight colors.

To toggle transparency:
- Press `Space` or `Enter` on **Transparent Background** in **Tab 2 (Settings)**.
- Or set `theme-transparent = true` in `%APPDATA%\llama-herd\config.toml` (Windows) or `~/.config/llama-herd/config.toml` (Linux/macOS).

---

## 🔄 Live Hot-Reloading

Themes in LlamaHerd reload dynamically in real time without restarting the application or dropping active inference sessions:
- **Interactive Switching**: Selecting any preset or toggling transparency in Tab 2 updates the UI palette in memory instantly and persists the change to `config.toml`.
- **Automatic File Watcher (`Custom` preset)**: When `Custom` is active, the background loop monitors `theme.toml` every second. Modifying and saving `theme.toml` in your editor immediately refreshes the active TUI display.

---

## 🖌️ Functional Palette Reference

The palette maps semantic roles across the application:

| Color | Purpose |
|-------|---------|
| `primary` | Main accent color, used for headers, active tab highlights, and primary borders. |
| `secondary` | Subdued color, used for labels and inactive elements. |
| `accent` | Highlight color for specific values like draft models or port numbers. |
| `selection` | Color for selected items in lists or active input fields. |
| `success` | Color indicating successful status, "ON" states, or valid parameters. |
| `error` | Color indicating errors, "OFF" states, or quit/cancel actions. |
| `bg` | Main background color. |
| `fg` | Main foreground (text) color. |
| `header-bg` | Background color specifically for the top header bar. |
| `footer-bg` | Background color specifically for the bottom hotkey hints bar. |

### Operational Status Badges

Server health states and badges dynamically inherit their colors from semantic palette entries:
- **`HEALTHY`**: Styled with `success` color.
- **`LOADING`**: Styled with `accent` color.
- **`UNHEALTHY` / `RECOVERING` / `ERROR`**: Styled with `error` color (`RECOVERING` is also bolded).
- **`STOPPED`**: Styled with `secondary` color.

---

## ⚙️ Custom Theme Configuration (`theme.toml`)

To craft a bespoke theme, select the **Custom** preset and edit `theme.toml` in your configuration directory:
- **Windows**: `%APPDATA%\llama-herd\theme.toml`
- **Linux/macOS**: `~/.config/llama-herd/theme.toml`

### `theme.toml` Schema (Reference)

```toml
[palette]
primary = "#83a598"
secondary = "#928374"
accent = "#fabd2f"
success = "#b8bb26"
error = "#fb4934"
selection = "#d3869b"
bg = "#282828"
fg = "#ebdbb2"
header-bg = "#32302f"
footer-bg = "#32302f"

[ui]
show-emojis = true
border-type = "plain" # "plain", "rounded", "double", "thick"
```

*Note: Colors support standard terminal color names (e.g., "light-blue", "black"), transparency aliases ("reset", "default", "none", "transparent"), hex codes (e.g., "#00ffff" or "#0ff"), and RGB/indexed formats (e.g., "rgb(20, 20, 20)", "indexed(234)").*
