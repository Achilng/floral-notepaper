# 花笺 by Atri

花笺 by Atri 是基于原开源项目 [floral-notepaper](https://github.com/Achilng/floral-notepaper) 修改的个人增强版桌面便签工具。

项目基于 **Tauri 2 + React** 构建，保留原项目轻量、优雅、本地化的特点，并根据个人日常记笔记和桌面提醒的使用习惯，增加了提醒中心、倒计时/闹钟、自定义铃声、背景图片和磁贴颜色等功能。

> 感谢原作者开源，本版本主要是个人使用场景下的功能增强版。

## 功能特点

- **Markdown 编辑与预览**  
  支持 GitHub Flavored Markdown，可在编辑和预览模式之间切换。

- **快捷便签 / 小窗模式**  
  支持通过托盘或全局快捷键快速唤出便签窗口，适合临时记录内容。

- **磁贴模式**  
  可将笔记固定在桌面上，方便快速查看、复制或作为待办清单使用。

- **本地提醒中心**  
  支持倒计时提醒、指定时间闹钟、完成提醒、稍后 5 分钟和绑定当前笔记。<img width="1769" height="1145" alt="屏幕截图 2026-05-22 000758" src="https://github.com/user-attachments/assets/00881d43-ddcb-4727-9aef-6521e0293ab4" />


- **小窗 / 磁贴提醒交互**  
  普通窗口下会弹出提醒窗口；小窗和磁贴模式下可直接在当前窗口中确认或稍后提醒。<img width="697" height="421" alt="屏幕截图 2026-05-22 000824" src="https://github.com/user-attachments/assets/e3b2980c-7a06-4b0b-989a-c3341d98a72b" /><img width="402" height="390" alt="屏幕截图 2026-05-22 000850" src="https://github.com/user-attachments/assets/b375b7b0-cf03-4012-a6f8-c308d66bd569" /><img width="422" height="409" alt="屏幕截图 2026-05-22 000909" src="https://github.com/user-attachments/assets/81ef1dd9-ffe3-4061-945d-847d0a506003" />




- **自定义铃声**  
  支持选择本地音频作为提醒铃声，提醒到期后循环播放，直到用户处理提醒。


- **倒计时显示**  
  绑定提醒的笔记会显示类似 `00:05:00` 的倒计时，主窗口、小窗和磁贴状态下均可查看。

- **自定义背景图片**  
  支持选择本地图片作为背景，并可调整填充方式、缩放、位置和遮罩强度。<img width="1787" height="1173" alt="屏幕截图 2026-05-21 235445" src="https://github.com/user-attachments/assets/68d18793-5c74-477d-9f6b-801f610dd63d" />


- **磁贴颜色增强**  
  自定义颜色可以实际应用到小窗和磁贴界面，不再只跟随深色/浅色主题。

- **导入导出**  
  支持 `.md` 文件导入和导出。

## 应用场景

- 临时记录灵感、想法和任务
- 桌面待办清单
- 学习、写作、看视频时作为悬浮笔记
- 作为轻量剪贴板暂存常用文本
- 设置短时间倒计时或定时提醒
- 将重要笔记固定为桌面磁贴

## 下载安装

前往本项目 Releases 页面下载安装包：

[GitHub Releases](https://github.com/atrilee0705-netizen/floral-notepaper-by-atri/releases)

目前主要在 **Windows 11** 上测试，其他系统兼容性尚未充分验证。

## 从源码构建

### 环境要求

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri CLI 2](https://tauri.app/)

### 克隆项目

```bash
git clone https://github.com/atrilee0705-netizen/floral-notepaper-by-atri.git
cd floral-notepaper-by-atri
```

如需使用当前增强功能分支：

```bash
git checkout feature/reminders-background
```

### 安装依赖

```bash
npm install
```

### 开发模式

```bash
npm run tauri dev
```

### 构建发布版本

```bash
npm run tauri build
```

构建产物一般位于：

```text
src-tauri/target/release/bundle/
```

Windows 安装包一般位于：

```text
src-tauri/target/release/bundle/nsis/
```

## 与原项目的关系

本项目是基于 [Achilng/floral-notepaper](https://github.com/Achilng/floral-notepaper) 的 fork 修改版本。

原项目提供了优秀的轻量便签基础框架，本版本主要增加了提醒、闹钟、背景图片、磁贴颜色和小窗交互等个人增强功能。如需查看原始版本，请访问原项目仓库。

## 已知限制

- 提醒功能为本地提醒，不支持云同步
- 应用完全退出后无法像系统闹钟一样继续后台提醒
- 自定义铃声和背景图片依赖本地文件
- 主要在 Windows 11 下测试

## 许可证

本项目沿用原项目许可证：

[MIT](LICENSE)
