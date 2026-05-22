# Configuration, Reference, & Performance Guide

This guide details all available configuration variables, parameters, API endpoints, advanced usage examples, and performance optimization techniques for `Llama-Herd`.

---

## Environment Variables

- `LLAMA_PATH`: The location containing the `llama-server` binary, the `models/` folder, the global `config.toml` and the generated `models-preset.ini`.
- `TERM`: Defines the terminal capabilities. Needs to be set to a 256-color profile (e.g., `xterm-256color` or `screen-256color`) to properly display TUI borders and styled ANSI logs.

---

## Global Configuration (`config.toml`)

Placed directly inside the `LLAMA_PATH` directory to define global settings shared across all presets.

| Parameter     | Default          | Type    | Description                                                                |
| :------------ | :--------------- | :------ | :------------------------------------------------------------------------- |
| `host`        | `"0.0.0.0"`      | String  | Host binding IP for `llama-server`.                                        |
| `port`        | `8080`           | Integer | Listen port for incoming inference requests.                               |
| `flash-attn`  | `"auto"`         | String  | Enables flash attention processing (`"auto"`, `"1"`, or `"0"`).            |
| `kv-quant`    | `"q8_0"`         | String  | Configures KV cache quantization type (`"q8_0"`, `"f16"`, `"q4_0"`, etc.). |
| `models-max`  | `1`              | Integer | Max loaded models concurrently hosted in Router Mode.                      |
| `batch-size`  | `256`            | Integer | Processing batch size (`-b`).                                              |
| `ubatch-size` | `256`            | Integer | Processing micro-batch size (`-ub`).                                       |
| `threads`     | _Physical Cores_ | Integer | Thread count allocation (`-t`). Defaults to physical hardware threads.     |
| `ui`          | `true`           | Boolean | Enable/Disable standard Web UI host wrapper.                               |

---

## Model-Specific Configuration (`<model-name>.toml`)

Configured next to a `.gguf` file (e.g. `Qwen2.5-7B-Instruct.toml` for `Qwen2.5-7B-Instruct.gguf`).

### TOML Key Naming Rules

1. Keys must not contain underscores (`_`). Use hyphens instead.
2. Keys must not start with a dash (`-`).
3. Keys that violate these rules are ignored at parse-time to guarantee command line safety.

### Configuration Key Prefixes

- **Prefix `lh-`**: Custom parameters processed internally by `llama-herd` and excluded from `llama-server` CLI arguments.
- **Prefix `s-`**: Mapped to short options for `llama-server`. For example, `s-sps = 0.5` translates to `-sps 0.5` (direct launch) or `sps = 0.5` (in the preset file).
- **Unprefixed (Normal) Keys**: Passed directly to `llama-server` as double-dash options. For example, `slot-prompt-similarity` becomes `--slot-prompt-similarity`.

### List of `lh-` Configuration Keys

| Key                                | Default  | Type       | Description                                                                                        |
| :--------------------------------- | :------- | :--------- | :------------------------------------------------------------------------------------------------- |
| `lh-is-draft` / `lh-is-draft-only` | `false`  | Boolean    | Designates the GGUF file as a speculative draft (hides it from the primary selections).            |
| `lh-is-default`                    | `false`  | Boolean    | Declares this model the default startup preset.                                                    |
| `lh-draft` / `lh-draft-model`      | `none`   | String     | Specific draft model file to pair with (use `"none"` or `"false"` to block pairing).               |
| `lh-mmproj`                        | `none`   | String     | Explicit vision projector filename to couple with this model.                                      |
| `lh-ctx-size`                      | `none`   | String/Int | Overrides context size (supports standard human shorthand: e.g., `"8k"`, `"32k"`).                 |
| `lh-ngl`                           | `none`   | String/Int | GPU offloaded layers count (supports `"auto"`).                                                    |
| `lh-total-layers`                  | `none`   | Integer    | Total structural layers of the neural network (used to resolve `"auto"` offloading).               |
| `lh-temp`                          | `0.8`    | Float      | Fallback model temperature parameter.                                                              |
| `lh-top-p`                         | `0.95`   | Float      | Top-p sampling probability limit.                                                                  |
| `lh-top-k`                         | `40`     | Integer    | Top-k sampling candidate count.                                                                    |
| `lh-reasoning`                     | `"auto"` | String     | Controls formatting for reasoning outputs (`"on"` maps to deepseek formats, `"off"`, or `"auto"`). |
| `lh-kv-quant`                      | `"q8_0"` | String     | KV quantization override (`"q8_0"`, `"q4_0"`, etc.).                                               |
| `lh-spec-type`                     | `none`   | String     | Speculative decoding mode (`"draft-mtp"`, `"draft-simple"`, `"draft-eagle3"`).                     |
| `lh-spec-draft-n-max`              | `4`      | Integer    | Max speculative draft token predictions per slots.                                                 |
| `lh-spec-draft-p-min`              | `0.0`    | Float      | Minimum probability threshold for speculative tokens.                                              |

---

## API Endpoint Overview (Router Mode)

When started in **Router Mode** (using `llama-herd --cli` -> Mode 1, or through the TUI -> `Ctrl + R`), `llama-server` functions as a dynamic gateway. It coordinates the lifecycle of multiple model presets based on client calls:

```
[Client App] ---> (POST /v1/chat/completions { "model": "qwen2-5-7b-instruct-draft" })
                       |
                       v
         [llama-server preset router]
         (Loads model and/or draft if not active, unloads oldest model if models-max exceeded)
                       |
                       v
         [Returns completion stream]
```

### Key Routing Endpoints

- `POST /v1/chat/completions`: Standard OpenAI Chat Completion endpoint. Dynamically loads the model specified in the `model` request payload, unloading other presets if `models-max` is exceeded.
- `GET /v1/models`: Returns a JSON listing of all available presets loaded from the generated `models-preset.ini` file.
- `POST /slots`: Returns diagnostic status information on available server slots and current active allocations.

---

## Advanced Configuration Examples

### 1. Model Configuration File (`Qwen2.5-7B-Instruct.toml`)

This configuration enables speculative decoding with a matching draft model, overrides the context size to `32k`, maps GPU layer settings, and forwards custom sampler options.

```toml
# Qwen2.5-7B-Instruct.toml
# Placed next to Qwen2.5-7B-Instruct.gguf

# Llama-Herd Orchestration Settings
lh-is-default = true
lh-ctx-size = "32k"
lh-total-layers = 28
lh-ngl = "auto"
lh-reasoning = "on"

# Speculative Decoding Configurations
lh-draft = "Qwen2.5-1.5B-Instruct.gguf"
lh-spec-type = "draft-mtp"
lh-spec-draft-n-max = 4
lh-spec-draft-p-min = 0.85

# llama-server Parameter Passthrough
s-sps = 0.6                     # Translates to short-arg -sps 0.6
slot-prompt-similarity = 0.5   # Translates to long-arg --slot-prompt-similarity 0.5
```

### 2. Auto-Generated `models-preset.ini`

On execution, Llama-Herd parses models and local configurations to output `models-preset.ini` in the `LLAMA_PATH` directory. Below is an example of what is generated:

```ini
version = 1
; Global settings shared across all presets
[*]
flash-attn = auto
jinja = true
cache-type-k = q8_0
cache-type-v = q8_0
kv-unified = true

; --- qwen2-5-7b-instruct ---
[qwen2-5-7b-instruct]
model = /llama/models/Qwen2.5-7B-Instruct.gguf
ctx-size = 32768
n-gpu-layers = 28
temp = 0.8
top-p = 0.95
top-k = 40
reasoning = on
reasoning-format = deepseek
sps = 0.6
slot-prompt-similarity = 0.5

; --- qwen2-5-7b-instruct-draft ---
[qwen2-5-7b-instruct-draft]
model = /llama/models/Qwen2.5-7B-Instruct.gguf
ctx-size = 32768
n-gpu-layers = 28
temp = 0.8
top-p = 0.95
top-k = 40
reasoning = on
reasoning-format = deepseek
sps = 0.6
slot-prompt-similarity = 0.5
model-draft = /llama/models/Qwen2.5-1.5B-Instruct.gguf
spec-type = draft-mtp
spec-draft-n-max = 4
spec-draft-p-min = 0.85
gpu-layers-draft = 4

[default]
model = /llama/models/Qwen2.5-7B-Instruct.gguf
ctx-size = 32768
n-gpu-layers = 28
temp = 0.8
top-p = 0.95
top-k = 40
reasoning = on
reasoning-format = deepseek
sps = 0.6
slot-prompt-similarity = 0.5
```

---

## Performance & Optimization

### GPU Offloading (VRAM)

GPU offloading is controlled via the `lh-ngl` option (and `gpu-layers-draft` for drafts). By default, setting `lh-ngl = "auto"` automatically resolves layers based on `lh-total-layers`. If you hit out-of-memory errors on GPU execution, you can specify negative deltas to allocate remaining layers onto the CPU.

- _Example_: `lh-ngl = "--4"` on a 32-layer model will assign `28` layers to the GPU and offload `4` layers to the system RAM.

### KV Cache Quantization

By default, standard FP16 KV caches utilize substantial memory as context grows. Enforcing `lh-kv-quant = "q8_0"` (or `"q4_0"`) inside the model configuration optimizes memory consumption:

- **`q8_0`**: 50% memory reduction in KV Cache allocation with minimal perplexity degradation.
- **`q4_0`**: ~75% memory reduction, enabling context lengths of up to `128k` on smaller consumer GPUs.

### Speculative Decoding Throughput

Pairing a smaller draft model with a larger primary model (using `lh-draft` and `lh-spec-type = "draft-mtp"`) increases inference token generation throughput by running predictions on the draft model and validating them in parallel on the primary model. Speculative parameters can be optimized using:

- **`lh-spec-draft-n-max`**: Standard value of `4` to `8`. Higher values check more tokens but can cause performance penalties if acceptance rates are low.
- **`lh-spec-draft-p-min`**: Set to `0.80 - 0.90` to restrict predictions only to highly probable tokens, increasing acceptance rates.
