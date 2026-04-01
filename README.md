# vibeKidbright

vibeKidbright is a modern, AI-powered Integrated Development Environment (IDE) specifically designed for ESP-IDF (Espressif IoT Development Framework) projects. Built with Tauri, React, and TypeScript, it provides a sleek, high-performance interface for developing, building, and flashing firmware to ESP32 microcontrollers.

## ✨ Features

- **Project Management**: Create new ESP-IDF projects or open existing ones.
- **Vibe Coder (AI Assistant)**: An integrated AI chat panel that can read your code, suggest improvements, and even inject code directly into your files.
- **Interactive Terminal**: A built-in terminal for running shell commands and viewing real-time logs from the ESP-IDF build system.
- **One-Click Build & Flash**: Easily compile your project and flash it to your device with a single button click.
- **Modern UI**: A premium, dark-themed interface built with Tailwind CSS for a smooth and productive developer experience.

## 🚀 Getting Started

### Prerequisites

Before you begin, ensure you have the following installed on your system:

- **Node.js** (v18 or later)
- **Rust** (and the Tauri CLI: `cargo install tauri-cli`)
- **ESP-IDF** (Properly installed and configured in your environment variables)
- **Xcode Command Line Tools** (For macOS users)

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/your-repo/vibeKidbright.git
   cd vibeKidbright
   ```

2. Install dependencies:
   ```bash
   npm install
   ```

### Development (Building Dev)

To start the development server and run the application in dev mode:

```bash
npm run tauri dev
```

This will launch the Vite dev server for the frontend and the Tauri-Rust backend in a desktop window.

### Production Build

To build the application for production:

```bash
npm run build
npm run tauri build
```

The executable will be located in `src-tauri/target/release/bundle/`.

## 🛠️ Technologies

- **Frontend**: [React](https://reactjs.org/), [TypeScript](https://www.typescriptlang.org/), [Vite](https://vitejs.dev/)
- **Styling**: [Tailwind CSS](https://tailwindcss.com/)
- **Backend/Desktop Layer**: [Tauri](https://tauri.app/), [Rust](https://www.rust-lang.org/)
- **Embedded Integration**: [ESP-IDF](https://docs.espressif.com/projects/esp-idf/en/latest/esp32/)

## 📝 License

This project is licensed under the MIT License.
