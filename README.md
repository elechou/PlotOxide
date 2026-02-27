# PlotDigitizer

A native, high-performance plot digitizer built with **Rust** and **egui**. Load an image of a chart or plot, calibrate the axes, click on data points, and export the extracted coordinates to CSV.

## Features

- Load images via file dialog, drag-and-drop, or clipboard paste
- 4-point axis calibration with support for linear and logarithmic scales
- Multiple data series (groups) with distinct colors
- Export extracted data to CSV
- Undo / Redo support
- Dark and light themes
- Cross-platform (Linux, macOS, Windows)

## Prerequisites

### 1. Install Rust

If you don't have Rust installed, use [rustup](https://rustup.rs/) (the official installer):

```bash
# Linux / macOS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Windows
# Download and run the installer from https://rustup.rs/
```

After installation, make sure `cargo` is available:

```bash
rustc --version
cargo --version
```

> **Tip:** If the commands are not found, restart your terminal or run `source $HOME/.cargo/env`.

### 2. Install system dependencies

**Linux (Debian / Ubuntu)**

```bash
sudo apt-get update
sudo apt-get install -y libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
    libxkbcommon-dev libssl-dev libgtk-3-dev
```

**macOS** – No extra dependencies are needed (Xcode Command Line Tools are sufficient):

```bash
xcode-select --install
```

**Windows** – No extra dependencies are needed.

## Building the Project

### Clone the repository

```bash
git clone https://github.com/elechou/PlotDigitizer.git
cd PlotDigitizer
```

### Build (debug)

```bash
cargo build
```

### Build (release, optimized)

```bash
cargo build --release
```

The compiled binary will be located at:

- Debug: `target/debug/plot-digitizer`
- Release: `target/release/plot-digitizer`

## Running

```bash
# Debug build
cargo run

# Release build (recommended for everyday use)
cargo run --release
```

## How to Use

### Step 1 – Load an image

Open the application and load a plot image using one of the following methods:

- Click the **Load Image** button in the top panel
- **Paste** an image from the clipboard (`Ctrl+V` / `Cmd+V` or click "Paste Image")
- Drag and drop an image file onto the window

A sample image (`sample_plot.png`) is included in the repository for testing.

### Step 2 – Calibrate the axes

1. Switch to **Add Calib** mode using the toolbar on the left side of the canvas.
2. Click on **4 known reference points** on the image — two along the X-axis and two along the Y-axis.
3. In the **Calibration** section of the left sidebar, enter the real-world values for each reference point (X₁, X₂, Y₁, Y₂).
4. If an axis uses a logarithmic scale, enable the **Log** checkbox next to that axis.

### Step 3 – Extract data points

1. Switch to **Add Data** mode using the toolbar.
2. Click on data points in the plot to mark them. Each click places a new point.
3. The extracted coordinates (mapped from pixels to real-world values) appear in the left sidebar.

### Step 4 – Organize data series

- Use the **Groups** section in the left sidebar to create, rename, and color-code different data series.
- Drag and drop points between groups to organize them.

### Step 5 – Export to CSV

Click **Save CSV** to export all extracted data. The output format is:

```
Group,X,Y
"20kHz",4.43354258,3.32255880
"20kHz",5.00902534,3.56703689
"30kHz",6.15800585,5.74756443
```

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

## Project Structure

```
PlotDigitizer/
├── src/
│   ├── main.rs        # Application entry point
│   ├── core.rs        # Calibration math and coordinate mapping
│   ├── state.rs       # Application state with undo/redo
│   ├── action.rs      # Action definitions for state updates
│   └── ui/
│       ├── mod.rs     # Main UI layout
│       ├── panel.rs   # Left sidebar (calibration, groups, data)
│       ├── canvas.rs  # Image viewport and interaction
│       └── toolbar.rs # Mode toolbar (Select, Add, Delete, Pan)
├── Cargo.toml         # Dependencies and build configuration
├── sample_plot.png    # Example plot image
└── extracted_data.csv # Example CSV output
```

## License

See the repository for license details.
