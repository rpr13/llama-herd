# Hybrid Theme System

LlamaHerd uses a Hybrid Theme System that combines a **Functional Palette** with **Procedural UI** behaviors. This allows for a flexible TUI that can adapt to different terminal environments and user preferences.

## Functional Palette

The palette consists of several semantic colors that are used throughout the TUI:

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

## UI Behaviors

Beyond colors, the theme system controls structural and aesthetic behaviors:

- **`show-emojis`**: (boolean) Toggles the use of Unicode emojis in the header logo, tabs, and lists.
- **`border-type`**: Defines the style of borders used in windows and panels (supports `plain`, `rounded`, `double`, `thick`).

## Configuration

To apply a custom theme, create a `theme.toml` file in your configuration directory:
- **Unix**: `~/.config/llama-herd/theme.toml`
- **Windows**: `%APPDATA%\llama-herd\theme.toml`

### `theme.toml` Schema (Reference)

```toml
[palette]
primary = "cyan"
secondary = "gray"
accent = "yellow"
success = "green"
error = "red"
selection = "magenta"
bg = "black"
fg = "white"
header-bg = "indexed(234)"
footer-bg = "indexed(234)"

[ui]
show-emojis = true
border-type = "plain"
```

*Note: Colors support standard terminal color names (e.g., "light-blue") and hex codes (e.g., "#00ffff" or "#0ff").*
