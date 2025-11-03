# 🚀 前端开发文档 - Classpaper v4

## 📋 项目概述

Classpaper v4 是一个基于 Web 技术的桌面壁纸课表应用，使用纯前端技术栈（HTML5 + CSS3 + JavaScript）构建，通过 Rust 后端提供系统托盘功能。

## 🏗️ 架构设计

### 整体架构
```
┌─────────────────────────────────────┐
│          Rust Backend              │
│        (系统托盘功能)               │
└─────────────────────────────────────┘
                │
┌─────────────────────────────────────┐
│         Web Frontend               │
│   ┌─────────────┐  ┌─────────────┐  │
│   │  HTML5      │  │  CSS3       │  │
│   │  (结构)     │  │  (样式)     │  │
│   └─────────────┘  └─────────────┘  │
│   ┌─────────────────────────────┐  │
│   │      JavaScript (逻辑)       │  │
│   │  ┌─────────┐  ┌─────────┐   │  │
│   │  │ 主逻辑   │  │ 课表逻辑 │   │  │
│   │  │ main.js │  │ class.js │   │  │
│   │  └─────────┘  └─────────┘   │  │
│   │  ┌─────────┐  ┌─────────┐   │  │
│   │  │ 日历    │  │ 帮助    │   │  │
│   │  │ event   │  │ help.js │   │  │
│   │  └─────────┘  └─────────┘   │  │
│   └─────────────────────────────┘  │
└─────────────────────────────────────┘
```

### 文件结构
```
res/
├── index.html          # 主页面
├── css/
│   ├── pico.css        # Pico CSS 框架
│   └── custom.css      # 自定义样式
├── js/
│   ├── main.js         # 主逻辑 (壁纸、时间、进度)
│   ├── class.js        # 课表逻辑
│   ├── event_cal.js    # 事件日历
│   ├── help.js         # 告示牌
│   └── api.js          # API 接口 (预留)
├── config/
│   └── config.js       # 全局配置
├── audio/              # 音频文件
└── wallpaper/          # 壁纸图片
```

## 🔌 暴露的 API 接口

### 🌍 全局对象

#### CONFIG 对象
**文件**: `res/config/config.js`
**描述**: 全局配置对象，包含所有应用配置

```javascript
// 访问方式
window.CONFIG.lessons.displayMode    // "scroll" 或 "day"
window.CONFIG.notifications.enabled    // 通知开关
window.CONFIG.wallpaperInterval        // 壁纸切换间隔(秒)
```

#### 数据变量
- `lessons`: 课表数据字符串
- `events`: 事件数据字符串  
- `wallpaperlist`: 壁纸数组
- `sth`: 告示牌内容

### 🎯 主逻辑 API (main.js)

#### 壁纸控制
```javascript
// 立即切换壁纸
window.changeWallpaper()

// 重新加载壁纸定时器
window.reloadWallpaperTimer()

// 获取当前壁纸索引
// 通过壁纸数组 wallpaperlist 访问
```

#### 时间相关
```javascript
// 手动更新时间显示
window.setTime()

// 获取当前周次
window.getYearWeek(new Date())

// 进度条更新
// 自动更新，可通过 CONFIG.progressDescription 自定义
```

### 📚 课表 API (class.js)

#### 核心功能
```javascript
// 刷新当前课程显示
window.nowClass()

// 获取今日课程数据
window.getDayVectors()
// 返回: {today_vec, prev_vec, next_vec}

// 播放通知音效
window.playNotification(type, className)
// type: "regular" | "ending"
```

#### 课程数据格式
```javascript
// 课程数据结构 (每日12节课)
const lessons = "星期,1,2,3,4,5,6,7,8,9,10,11\n,周一,语文,数学,..."

// 时间配置格式
CONFIG.lessons.times.schedule = [
  {
    period: 1,
    begin: "07:20",
    end: "07:55",
    rest: "07:55-08:00"
  }
]
```

### 🛠️ 调试 API (class.js)

ClassPaper v4 提供了完整的调试工具集，帮助开发者测试和验证课程表显示功能。

#### 时间相关调试
```javascript
// 测试当前时间显示
window.testCurrentTime()

// 模拟时间进度（自动进入调试模式）
window.simulateTimeProgress()

// 模拟下一步（需先调用simulateTimeProgress）
window.simulateNextStep()

// 退出调试模式，恢复自动刷新
window.exitDebugMode()
```

#### 课程显示测试
```javascript
// 测试滚动模式（自动进入调试模式）
window.testScrollMode()

// 测试日间模式（自动进入调试模式）
window.testDayMode()

// 测试课程排列（自动进入调试模式）
window.testArrangeClasses()
```

#### 模式切换
```javascript
// 切换显示模式（自动进入调试模式）
window.toggleDisplayMode()
```

#### 调试模式控制
```javascript
// 进入调试模式（暂停自动刷新）
window.enterDebugMode()

// 检查是否处于调试模式
window.isDebugMode()
```

#### 调试功能说明

- **自动进入调试模式**：所有测试函数会自动调用 `enterDebugMode()` 暂停自动刷新
- **恢复自动刷新**：使用 `exitDebugMode()` 恢复正常的自动刷新功能
- **状态提示**：进入调试模式后，控制台会显示恢复提示信息
- **时间模拟**：`simulateTimeProgress()` 会从早上7:00开始模拟一天的课程变化
- **课程显示测试**：测试函数会显示不同模式下的课程排列效果

#### 常见调试场景

1. **验证课程显示**：使用 `testScrollMode()` 和 `testDayMode()` 测试不同显示模式
2. **时间模拟**：使用 `simulateTimeProgress()` 模拟一天的课程变化
3. **课程排列测试**：使用 `testArrangeClasses()` 验证课程排列逻辑
4. **模式切换**：使用 `toggleDisplayMode()` 测试显示模式切换功能

#### 调试示例

```javascript
// 示例1：测试滚动模式
testScrollMode()  // 自动进入调试模式并显示滚动模式
// ... 查看效果
exitDebugMode()   // 恢复自动刷新

// 示例2：模拟时间进度
simulateTimeProgress()  // 开始时间模拟
// ... 观察时间变化和课程切换
simulateNextStep()      // 手动推进到下一步
exitDebugMode()         // 结束模拟，恢复自动刷新
```

### 📅 事件日历 API (event_cal.js)

#### 事件数据格式
```javascript
// 事件数据结构
const events = "事件,日期,\n高考,2026-06-07T00:00:00,\n考试,2025-01-15T00:00:00"

// 事件配置
CONFIG.events = [
  {
    name: "高考",
    date: "2026-06-07T00:00:00"
  }
]
```

### 🔔 通知系统

#### 通知配置
```javascript
CONFIG.notifications = {
  enabled: true,           // 总开关
  regularInterval: 5,      // 常规提醒间隔(分钟)
  endingTime: 5            // 下课提醒时间(分钟)
}
```

#### 音效文件
- `audio/regular_notification.mp3` - 常规提醒音效
- `audio/ending_notification.mp3` - 下课提醒音效

## 🎨 样式系统

### CSS 变量
```css
:root {
  /* 主题色 */
  --primary: #3daee9;
  --secondary: #93cee9;
  
  /* 透明度 */
  --glass-opacity: 0.8;
  --glass-blur: 10px;
}
```

### 响应式设计
- 支持多种屏幕分辨率
- 自适应壁纸显示
- 移动端友好

## 🛠️ 开发指南

### 快速开始

#### 1. 修改配置
```javascript
// 编辑 res/config/config.js
CONFIG.lessons.displayMode = "day";  // 切换显示模式
CONFIG.wallpaperInterval = 60;       // 修改壁纸间隔
```

#### 2. 添加新功能
```javascript
// 在 main.js 中添加新API
window.myNewFeature = function() {
  // 你的代码
}
```

#### 3. 自定义样式
```css
/* 在 custom.css 中添加 */
.my-custom-class {
  background: rgba(255, 255, 255, 0.1);
  backdrop-filter: blur(10px);
}
```

### 调试技巧

#### 浏览器调试
1. 直接打开 `res/index.html`
2. 按 F12 打开开发者工具
3. 在 Console 中测试 API:
```javascript
// 测试API
window.changeWallpaper()
window.nowClass()
console.log(CONFIG)
```

#### 调试API使用
ClassPaper v4 提供了专门的调试API，方便开发者测试和验证功能：

```javascript
// 测试课程显示模式
testScrollMode()  // 测试滚动模式
testDayMode()     // 测试日间模式

// 模拟时间进度
simulateTimeProgress()  // 从早上7:00开始模拟
simulateNextStep()      // 推进到下一步

// 测试完成后恢复自动刷新
exitDebugMode()
```

#### 日志输出
```javascript
// 所有模块都有日志输出
console.log("屏幕大小：1920*1080")
console.log("[课程定位] 高亮课程 语文")

// 调试模式下的额外日志
console.log("[调试模式] 已暂停自动刷新，使用 exitDebugMode() 恢复")
```

## 📊 数据流

### 配置加载流程
```
config.js → index.html → 各模块初始化
     ↓
CONFIG对象 → 全局可用
     ↓
lessons/events/wallpaperlist → 模块使用
```

### 更新机制
```
用户操作 → 修改CONFIG → 模块响应 → UI更新
定时器 → 自动刷新 → 数据检查 → 通知播放
```

## 🔧 扩展建议

### 可扩展功能
1. **主题切换**: 添加 CONFIG.theme 支持
2. **天气集成**: 添加天气API调用
3. **自定义布局**: 支持拖拽调整组件位置
4. **数据持久化**: 本地存储用户偏好

### API 扩展示例
```javascript
// 添加天气API
window.updateWeather = async function() {
  const weather = await fetchWeatherAPI();
  document.getElementById('weather').textContent = weather;
}

// 添加主题切换
window.switchTheme = function(theme) {
  document.documentElement.setAttribute('data-theme', theme);
}
```

## 📞 技术支持

### 常见问题
1. **配置不生效**: 检查浏览器缓存
2. **图片不显示**: 检查路径是否正确
3. **通知不响**: 检查音频文件是否存在

### 开发环境
- 浏览器: Chrome/Firefox/Edge
- 编辑器: VS Code (推荐)
- 调试: 浏览器开发者工具

---

💡 **提示**: 所有API都暴露在window对象下，可直接在浏览器控制台调用测试！

🔧 **调试提示**: 使用调试API时，系统会自动进入调试模式暂停自动刷新。完成测试后，请使用 `exitDebugMode()` 恢复自动刷新功能。