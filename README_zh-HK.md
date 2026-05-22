# 花箋 by Atri

花箋 by Atri 是基於原開源項目 [floral-notepaper](https://github.com/Achilng/floral-notepaper) 修改而來的個人增強版桌面便箋工具。

本項目基於 **Tauri 2 + React** 構建，保留原項目輕量、優雅、本地化的特點，並按照個人日常記筆記與桌面提醒的使用習慣，加入提醒中心、倒數計時／鬧鐘、自訂鈴聲、背景圖片和磁貼顏色等功能。

> 感謝原作者開源。本版本主要是面向個人使用場景的功能增強版。

## 功能特色

- **Markdown 編輯與預覽**  
  支援 GitHub Flavored Markdown，可在編輯和預覽模式之間切換。

- **快捷便箋 / 小窗模式**  
  支援透過托盤或全域快捷鍵快速叫出便箋視窗，適合臨時記錄內容。

- **磁貼模式**  
  可將筆記固定在桌面上，方便快速查看、複製，或作為輕量待辦清單使用。

- **本地提醒中心**  
  支援倒數計時提醒、指定時間鬧鐘、完成提醒、稍後 5 分鐘，以及綁定目前筆記。

<img width="1769" height="1145" alt="提醒中心截圖" src="https://github.com/user-attachments/assets/00881d43-ddcb-4727-9aef-6521e0293ab4" />

- **小窗 / 磁貼提醒互動**  
  普通視窗下會彈出提醒視窗；小窗和磁貼模式下可直接在目前視窗中確認或稍後提醒。

<img width="697" height="421" alt="提醒視窗截圖" src="https://github.com/user-attachments/assets/e3b2980c-7a06-4b0b-989a-c3341d98a72b" />

<img width="402" height="390" alt="小窗提醒截圖" src="https://github.com/user-attachments/assets/b375b7b0-cf03-4012-a6f8-c308d66bd569" />

<img width="422" height="409" alt="磁貼提醒截圖" src="https://github.com/user-attachments/assets/81ef1dd9-ffe3-4061-945d-847d0a506003" />

- **自訂鈴聲**  
  支援選擇本地音訊作為提醒鈴聲。提醒到期後會循環播放，直到使用者完成或稍後提醒。

- **倒數計時顯示**  
  綁定提醒的筆記會顯示類似 `00:05:00` 的倒數計時。主視窗、小窗和磁貼模式下皆可查看。

- **自訂背景圖片**  
  支援選擇本地圖片作為背景，並可調整填充方式、縮放、位置和遮罩強度。

<img width="1787" height="1173" alt="自訂背景截圖" src="https://github.com/user-attachments/assets/68d18793-5c74-477d-9f6b-801f610dd63d" />

- **磁貼顏色增強**  
  自訂顏色可以實際套用到小窗和磁貼介面，不再只跟隨深色／淺色主題。

- **匯入匯出**  
  支援 `.md` 檔案的匯入與匯出。

## 使用場景

- 臨時記錄靈感、想法和任務
- 桌面待辦清單
- 學習、寫作、看影片時作為懸浮筆記
- 作為輕量剪貼簿暫存常用文字
- 設定短時間倒數或定時提醒
- 將重要筆記固定為桌面磁貼

## 下載安裝

前往本項目的 Releases 頁面下載安裝包：

[GitHub Releases](https://github.com/atrilee0705-netizen/floral-notepaper-by-atri/releases)

目前主要在 **Windows 11** 上測試，其他系統的相容性尚未充分驗證。

## 從源碼構建

### 環境要求

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri CLI 2](https://tauri.app/)

### 克隆項目

```bash
git clone https://github.com/atrilee0705-netizen/floral-notepaper-by-atri.git
cd floral-notepaper-by-atri
```

如需使用目前的增強功能分支：

```bash
git checkout feature/reminders-background
```

### 安裝依賴

```bash
npm install
```

### 開發模式

```bash
npm run tauri dev
```

### 構建發布版本

```bash
npm run tauri build
```

構建產物通常位於：

```text
src-tauri/target/release/bundle/
```

Windows 安裝包通常位於：

```text
src-tauri/target/release/bundle/nsis/
```

## 與原項目的關係

本項目是基於 [Achilng/floral-notepaper](https://github.com/Achilng/floral-notepaper) 的 fork 修改版本。

原項目提供了優秀的輕量桌面便箋基礎框架。本版本主要加入提醒、鬧鐘、背景圖片、磁貼顏色和小窗互動等個人增強功能。

## 已知限制

- 提醒功能為本地提醒，不支援雲端同步
- 應用完全退出後，無法像系統鬧鐘一樣繼續在背景提醒
- 自訂鈴聲和背景圖片依賴本地檔案
- 主要在 Windows 11 下測試

## 授權

本項目沿用原項目授權：

[MIT](LICENSE)
