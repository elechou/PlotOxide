# PlotRedox

A native, high-performance plot digitizer built with **Rust** and **egui**. Load an image of a chart or plot, calibrate the axes, click on data points, and export the extracted coordinates to CSV — with a built-in **scripting IDE** for on-the-fly data analysis.

An open source alternative to GetData Graph Digitizer, WebPlotDigitizer, PlotDigitizer.

![PlotRedox Screenshot](screenshot.png)

## Features

### Core Digitizing
- Load images via file dialog, drag-and-drop, or clipboard paste
- 4-point axis calibration with support for linear and logarithmic scales
- Multiple data series (groups) with distinct colors and drag-and-drop organization
- Export extracted data to CSV
- Undo / Redo support
- Dark and light themes
- Cross-platform (Linux, macOS, Windows)

### Script IDE
- Built-in live scripting IDE powered by [Rhai](https://rhai.rs/) (Rust-embedded scripting language)
- **Script Templates** — pre-built examples ready to use:
  - Basic syntax & functions demo
  - Linear regression for all groups
  - Quadratic (polynomial) regression
  - Steinmetz parameter fitting (multi-variate OLS)
- **Workspace panel** — inspect all variables after script execution, click to view data tables
- **Import / Export** — save and load `.rhai` script files
- **Help** — built-in scripting reference with syntax guide and full API documentation

### Math & Analysis API
Scripts have access to a rich set of built-in functions:

| Category | Functions |
|----------|-----------|
| Math | `abs`, `sqrt`, `ln`, `log10`, `log2`, `exp`, `pow`, `pow10`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `floor`, `ceil`, `round`, `round_to`, `PI()` |
| Array | `sum`, `mean`, `min_val`, `max_val`, `std_dev`, `variance`, `log10_array` |
| Data | `col(array, "field")`, `extract_number(string)` |
| Regression | `linreg(x, y)`, `polyfit(x, y, degree)`, `lstsq(A, b)` |

## Installation

### Download Releases
You can download pre-compiled binaries for Windows, macOS, and Linux from the [Releases](https://github.com/elechou/PlotRedox/releases) page.

> [!IMPORTANT]
> **A Note on Security & Privacy:**
> Since these binaries are not signed with expensive developer certificates, you might encounter security warnings:
> - **Windows:** "Windows protected your PC" (SmartScreen). Click *More info* → *Run anyway*.
> - **macOS:** "App cannot be opened because the developer cannot be verified". Right-click the app and select *Open*, or go to *System Settings* → *Privacy & Security*.
>
> If you prefer not to bypass these warnings, you are encouraged to audit the source code and build the application yourself locally (see below).

## Building from Source

Building from source ensures you are running the exact code present in this repository. It is the recommended method for security-conscious users.

### 1. Install Rust
If you don't have Rust installed, visit [rustup.rs](https://rustup.rs/) or run:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. Install System Dependencies

**Linux (Debian / Ubuntu)**
```bash
sudo apt-get update
sudo apt-get install -y libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
    libxkbcommon-dev libssl-dev libgtk-3-dev
```

**macOS** – Xcode Command Line Tools are required: `xcode-select --install`

**Windows** – No extra dependencies are needed.

### 3. Build & Run
```bash
git clone https://github.com/elechou/PlotRedox.git
cd PlotRedox

# Run in optimized release mode
cargo run --release
```

The compiled binary will be located at `target/release/plot-redox` (or `.exe` on Windows).

## How to Use

### Step 1 – Load an image

Open the application and load a plot image:
- Click **Load Image** in the top toolbar
- **Paste** from clipboard (`Ctrl+V` / `Cmd+V` or click "Paste Image")
- Drag and drop an image file onto the window

A sample image (`sample_plot.png`) is included for testing.

### Step 2 – Calibrate the axes

1. Click **Place Calib Points** and click on **4 known reference points** on the image (two along X-axis, two along Y-axis).
2. Enter the real-world values for each reference point (X₁, X₂, Y₁, Y₂) in the left sidebar.
3. Enable **Log X** / **Log Y** checkboxes if an axis uses a logarithmic scale.

### Step 3 – Extract data points

1. Switch to **Add Data** mode using the canvas toolbar.
2. Click on data points in the plot. Extracted coordinates appear in the left sidebar.

### Step 4 – Organize & export

- Use **Groups** to create, rename, and color-code different data series.
- Drag and drop points between groups.
- Click **Export CSV** to save all data.

### Step 5 – Analyze with scripts

1. Click **Script IDE** in the top-right corner to open the IDE panel.
2. Select a template from **Script Templates** or write your own Rhai script.
3. Click **▶ Run Script** to execute. Results appear in the Output panel; variables appear in the Workspace panel.
4. Click **ⓘ Help** for the full API reference and syntax guide.

## Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `Ctrl+Z` / `Cmd+Z` | Undo |
| `Ctrl+Shift+Z` / `Cmd+Shift+Z` | Redo |
| `Delete` / `Backspace` | Delete selected points |
| `Arrow keys` | Nudge selected points |
| `Ctrl+V` / `Cmd+V` | Paste image from clipboard |
| `Escape` | Cancel current mode |
| `Shift+Click` | Range select |
| `Ctrl+Click` / `Cmd+Click` | Toggle individual selection |

## Customizing Built-in Scripts

PlotRedox features a powerful **auto-discovery system** for script templates. You are not limited to the presets provided in the repository; you can easily add your own permanent code snippets to the IDE.

### How it Works
The build system (`build.rs`) automatically scans the `example_scripts/` directory during compilation and embeds every `.rhai` file it finds directly into the application's binary. This means your personal analysis scripts will appear in the **Script Templates** menu just like the built-in ones.

### Adding Your Own Scripts
1.  Navigate to the `example_scripts/` folder in the root of the repo.
2.  Create a new `.rhai` file (e.g., `my_custom_filter.rhai`).
3.  **Re-compile** the application using `cargo run --release`.

#### Tips for Organization
- **Menu Order**: Files are sorted alphabetically. Use numeric prefixes like `01_load_data.rhai`, `02_clean_data.rhai` to control the exact order in the dropdown menu.
- **Display Names**: The system automatically "prettifies" filenames for the UI. For example, `03_advanced_log_fit.rhai` will be displayed as **"Advanced Log Fit"** in the IDE.

## Project Structure

```
PlotRedox/
├── src/
│   ├── main.rs           # Application entry point
│   ├── core.rs           # Calibration math and coordinate mapping
│   ├── state.rs          # Application state with undo/redo
│   ├── action.rs         # Action definitions for state updates
│   ├── ui/
│   │   ├── mod.rs        # Main UI layout and modals
│   │   ├── panel.rs      # Left sidebar (calibration, groups, data)
│   │   ├── canvas.rs     # Image viewport and interaction
│   │   └── toolbar.rs    # Mode toolbar (Select, Add, Delete, Pan)
│   ├── ide/
│   │   ├── mod.rs        # IDE panel layout
│   │   ├── editor.rs     # Code editor with syntax highlighting
│   │   ├── workspace.rs  # Variable inspector panel
│   │   ├── inspector.rs  # Data table viewer
│   │   ├── presets.rs    # Script templates and import/export
│   │   └── help.rs       # Built-in scripting reference
│   └── script/
│       ├── mod.rs        # Rhai engine setup and data binding
│       └── math.rs       # Math and regression functions
├── assets/
│   ├── icon.icns          # macOS app icon
│   ├── icon.ico           # Windows app icon
│   └── icon.png           # PNG icon (1024×1024)
├── example_scripts/       # Built-in script templates (.rhai)
├── docs/
│   └── scripting_help.md  # Scripting reference (embedded at build)
├── build.rs               # Auto-discovers example scripts & embeds Windows icon
├── Cargo.toml             # Dependencies and build configuration
├── sample_plot.png        # Example plot image
└── screenshot.png         # Application screenshot
```

## License

This project is licensed under the [MIT License](LICENSE).
