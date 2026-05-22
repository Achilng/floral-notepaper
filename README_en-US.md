# Floral Notepaper by Atri

Floral Notepaper by Atri is a personal enhanced version based on the open-source project [floral-notepaper](https://github.com/Achilng/floral-notepaper).

Built with **Tauri 2 + React**, this version keeps the original lightweight, elegant, and local-first desktop notepaper experience, while adding features that better fit my personal note-taking and desktop reminder workflow, including a reminder center, countdowns/alarms, custom sounds, background images, and tile color customization.

> Thanks to the original author for open-sourcing this project. This version is mainly a personal enhanced build for my own daily use.

## Features

- **Markdown Editing and Preview**  
  Supports GitHub Flavored Markdown and switching between editing and preview modes.

- **Quick Notes / Small Window Mode**  
  Quickly open a note window from the tray or global shortcut, suitable for temporary notes.

- **Tile Mode**  
  Pin notes to the desktop for quick viewing, copying, or use as a lightweight todo list.

- **Local Reminder Center**  
  Supports countdown reminders, scheduled alarms, completing reminders, snoozing for 5 minutes, and binding reminders to the current note.

<img width="1769" height="1145" alt="Reminder Center Screenshot" src="https://github.com/user-attachments/assets/00881d43-ddcb-4727-9aef-6521e0293ab4" />

- **Small Window / Tile Reminder Interaction**  
  In the main window, reminders appear as a centered reminder window. In small window or tile mode, users can confirm or snooze reminders directly inside the current window.

<img width="697" height="421" alt="Reminder Window Screenshot" src="https://github.com/user-attachments/assets/e3b2980c-7a06-4b0b-989a-c3341d98a72b" />

<img width="402" height="390" alt="Small Window Reminder Screenshot" src="https://github.com/user-attachments/assets/b375b7b0-cf03-4012-a6f8-c308d66bd569" />

<img width="422" height="409" alt="Tile Reminder Screenshot" src="https://github.com/user-attachments/assets/81ef1dd9-ffe3-4061-945d-847d0a506003" />

- **Custom Reminder Sound**  
  Supports selecting local audio files as reminder sounds. The sound loops until the reminder is completed or snoozed.

- **Countdown Display**  
  Notes linked to reminders can display a countdown such as `00:05:00`. It works in the main window, small window, and tile mode.

- **Custom Background Image**  
  Supports selecting a local image as the UI background, with options for fit mode, scale, position, and overlay intensity.

<img width="1787" height="1173" alt="Custom Background Screenshot" src="https://github.com/user-attachments/assets/68d18793-5c74-477d-9f6b-801f610dd63d" />

- **Enhanced Tile Color Customization**  
  Custom colors can be applied to small windows and tiles instead of only following the light/dark theme.

- **Import and Export**  
  Supports importing and exporting `.md` files.

## Use Cases

- Quickly capture ideas, tasks, and temporary notes
- Desktop todo list
- Floating notes while studying, writing, watching videos, or gaming
- Lightweight clipboard for frequently used text
- Short countdowns and scheduled reminders
- Pin important notes as desktop tiles

## Download

Download the installer from the Releases page:

[GitHub Releases](https://github.com/atrilee0705-netizen/floral-notepaper-by-atri/releases)

This version has mainly been tested on **Windows 11**. Compatibility with other systems has not been fully verified.

## Build from Source

### Requirements

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri CLI 2](https://tauri.app/)

### Clone

```bash
git clone https://github.com/atrilee0705-netizen/floral-notepaper-by-atri.git
cd floral-notepaper-by-atri
```

Use the enhanced feature branch:

```bash
git checkout feature/reminders-background
```

### Install Dependencies

```bash
npm install
```

### Development

```bash
npm run tauri dev
```

### Build

```bash
npm run tauri build
```

Build outputs are usually located in:

```text
src-tauri/target/release/bundle/
```

The Windows installer is usually located in:

```text
src-tauri/target/release/bundle/nsis/
```

## Relationship with the Original Project

This project is a fork of [Achilng/floral-notepaper](https://github.com/Achilng/floral-notepaper).

The original project provides an excellent lightweight desktop notepaper foundation. This version mainly adds personal enhancements such as reminders, alarms, background images, tile colors, and small window interactions.

## Known Limitations

- Reminders are local only and do not support cloud sync
- If the app is fully closed, it cannot continue running like a system alarm
- Custom sounds and background images depend on local files
- Mainly tested on Windows 11

## License

This project follows the original project's license:

[MIT](LICENSE)
