# ClassPaper 前端界面

## 概述

这是一个基于现有配置的现代化前端界面，支持直接在浏览器中调试，无需启动 Rust 应用程序。

## 特性

### 🎨 现代化设计
- 采用 Apple 和 Fluent UI 设计风格
- 渐变背景和毛玻璃效果
- 响应式布局，支持移动设备
- 优雅的动画和过渡效果

### 📅 课程表功能
- 实时显示当前课程（高亮显示）
- 支持滚动和网格两种显示模式
- 完整的周课程安排
- 时间段自动计算

### 📊 进度追踪
- 学期进度可视化
- 剩余天数计算
- 可自定义进度描述

### 🎯 事件管理
- 重要事件倒计时
- 自动排序和过滤
- 高考等重要日期提醒

### 🖼️ 壁纸系统
- 自动壁纸轮播
- 可自定义切换间隔
- 支持多种图片格式

### ⚙️ 设置界面
- 完整的配置管理
- 实时预览和保存
- 课程表可视化编辑
- 壁纸选择器

## 文件结构

```
res/
├── index.html          # 主界面
├── settings.html       # 设置界面
├── js/
│   ├── main.js         # 主界面逻辑
│   └── settings.js     # 设置界面逻辑
├── config/
│   └── config.js       # 配置文件（现有）
├── wallpaper/          # 壁纸文件夹（现有）
└── audio/              # 音频文件（现有）
```

## 使用方法

### 1. 直接浏览器调试
```bash
# 进入项目目录
cd d:\Documents\GitHub\Classpaper-v4\res

# 使用任意 HTTP 服务器（推荐）
# 例如使用 Python:
python -m http.server 8000

# 或使用 Node.js:
npx http-server

# 然后访问 http://localhost:8000
```

### 2. 文件直接打开
也可以直接在浏览器中打开 `index.html`，但某些功能可能受限。

### 3. 与 Rust 应用集成
Rust 应用会自动加载 `index.html` 作为主界面。

## 快捷键

- `Esc`: 关闭设置窗口
- `Ctrl + R`: 刷新数据
- `Ctrl + ,`: 打开设置
- `Ctrl + S`: 保存设置（在设置页面）

## 配置说明

所有配置都基于现有的 `config/config.js` 文件，支持以下主要配置项：

### 课程表配置
- `lessons.headers`: 表头（星期和时间段）
- `lessons.schedule`: 每日课程安排
- `lessons.times`: 时间段定义
- `lessons.displayMode`: 显示模式（scroll/grid）

### 外观配置
- `wallpapers`: 壁纸文件列表
- `wallpaperInterval`: 壁纸切换间隔（秒）
- `progressDescription`: 进度描述文本

### 功能配置
- `weekOffset`: 周偏移量
- `notifications`: 通知设置
- `events`: 重要事件列表
- `note`: 备注信息

## 浏览器兼容性

- Chrome 80+
- Firefox 75+
- Safari 13+
- Edge 80+

## 技术栈

- **HTML5 + CSS3**: 现代化样式和布局
- **Vanilla JavaScript**: 无依赖，轻量级
- **Font Awesome**: 图标库
- **Google Fonts**: 字体库

## 更新日志

### v1.0.0 (当前)
- ✨ 初始版本发布
- 🎨 现代化 UI 设计
- 📅 完整的课程表功能
- ⚙️ 可视化设置界面
- 🖼️ 壁纸轮播系统
- 📊 进度追踪功能

## 贡献指南

欢迎提交 Issue 和 Pull Request 来改进这个前端界面。

## 许可证

与主项目保持一致。