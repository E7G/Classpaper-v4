// ClassPaper 核心库 - 消除所有重复代码和全局依赖
class ClassPaperCore {
  constructor() {
    this.config = null;
    this.callbacks = new Map();
    this.initialized = false;
  }

  // 单一数据源 - 消除配置重复
  async loadConfig() {
    try {
      if (typeof CONFIG !== 'undefined') {
        this.config = this.validateConfig(CONFIG);
        this.emit('config:loaded', this.config);
        return this.config;
      }
      
      // 如果CONFIG不存在，尝试从文件加载
      const response = await fetch('config/config.js');
      const text = await response.text();
      
      // 安全地执行配置脚本
      const func = new Function(text + '; return CONFIG;');
      const config = func();
      this.config = this.validateConfig(config);
      this.emit('config:loaded', this.config);
      return this.config;
    } catch (error) {
      console.error('配置加载失败:', error);
      this.emit('config:error', error);
      return this.getDefaultConfig();
    }
  }

  // 配置验证 - 消除运行时错误
  validateConfig(config) {
    const required = ['lessons', 'wallpapers', 'events'];
    const validated = { ...config };
    
    for (const key of required) {
      if (!validated[key]) {
        console.warn(`配置缺少必需字段: ${key}`);
        validated[key] = this.getDefaultConfig()[key];
      }
    }
    
    return validated;
  }

  // 默认配置 - 消除undefined错误
  getDefaultConfig() {
    return {
      lessons: {
        headers: ['星期', '1', '2', '3', '4', '5', '6', '7', '8', '9', '10', '11'],
        schedule: [],
        times: { semester: {}, schedule: [] }
      },
      wallpapers: [],
      events: [],
      notifications: { enabled: false },
      weekOffset: { enabled: false, offset: 0 },
      wallpaperInterval: 30,
      progressDescription: '学习进度',
      progressPercentMode: 'left',
      note: ''
    };
  }

  // 事件系统 - 消除重复事件处理
  on(event, callback) {
    if (!this.callbacks.has(event)) {
      this.callbacks.set(event, []);
    }
    this.callbacks.get(event).push(callback);
  }

  emit(event, data) {
    if (this.callbacks.has(event)) {
      this.callbacks.get(event).forEach(callback => callback(data));
    }
  }

  // 工具函数 - 消除重复工具代码
  static formatDate(date, options = {}) {
    const opts = { year: 'numeric', month: 'long', day: 'numeric', weekday: 'long', ...options };
    return date.toLocaleDateString('zh-CN', opts);
  }

  static getDayName(date) {
    const days = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];
    return days[date.getDay()];
  }

  static calculateDaysBetween(start, end) {
    return Math.ceil((end - start) / (1000 * 60 * 60 * 24));
  }

  static getCurrentTimeInMinutes() {
    const now = new Date();
    return now.getHours() * 60 + now.getMinutes();
  }

  static parseTimeString(timeStr) {
    const [hours, minutes] = timeStr.split(':').map(Number);
    return hours * 60 + minutes;
  }
}

// 课程表管理器 - 消除重复课程表逻辑
class ScheduleManager {
  constructor(core) {
    this.core = core;
    this.currentPeriod = null;
  }

  // 获取当前课程时段 - 消除重复时间计算
  getCurrentPeriod() {
    const config = this.core.config;
    if (!config?.lessons?.times?.schedule) return null;

    const currentTime = ClassPaperCore.getCurrentTimeInMinutes();
    const schedule = config.lessons.times.schedule;

    for (let i = 0; i < schedule.length; i++) {
      const period = schedule[i];
      const beginTime = ClassPaperCore.parseTimeString(period.begin);
      const endTime = ClassPaperCore.parseTimeString(period.end);

      if (currentTime >= beginTime && currentTime <= endTime) {
        return { period: i + 1, ...period };
      }
    }

    return null;
  }

  // 获取今天的课程 - 消除重复日期计算
  getTodaySchedule() {
    const config = this.core.config;
    if (!config?.lessons?.schedule) return null;

    const todayName = ClassPaperCore.getDayName(new Date());
    return config.lessons.schedule.find(day => day.day === todayName);
  }

  // 获取当前课程信息 - 消除重复逻辑
  getCurrentClass() {
    const period = this.getCurrentPeriod();
    const todaySchedule = this.getTodaySchedule();

    if (!period || !todaySchedule) return null;

    return {
      period: period.period,
      className: todaySchedule.classes[period.period - 1],
      time: period
    };
  }

  // 获取今日相关课程 - 显示全天课程但保持信息密度
  getTodayRelevantCourses() {
    const config = this.core.config;
    if (!config?.lessons?.schedule || !config?.lessons?.times?.schedule) return [];

    const todaySchedule = this.getTodaySchedule();
    const currentPeriod = this.getCurrentPeriod();
    const allPeriods = config.lessons.times.schedule;

    if (!todaySchedule || !allPeriods.length) return [];

    // 显示全天课程，但优化信息密度
    const relevantCourses = [];
    for (let i = 0; i < allPeriods.length; i++) {
      const period = allPeriods[i];
      const className = todaySchedule.classes[i];
      
      // 只显示有课程的时段，消除空时段的信息噪声
      if (className && className.trim() !== '' && className !== '无') {
        relevantCourses.push({
          period: i + 1,
          time: period,
          className: className,
          isCurrent: currentPeriod && currentPeriod.period === i + 1,
          isNext: currentPeriod && currentPeriod.period + 1 === i + 1,
          isPast: currentPeriod && currentPeriod.period > i + 1
        });
      }
    }

    return relevantCourses;
  }
}

// 壁纸管理器 - 消除重复壁纸逻辑
class WallpaperManager {
  constructor(core) {
    this.core = core;
    this.currentIndex = 0;
    this.interval = null;
  }

  // 开始壁纸轮播 - 消除重复定时器逻辑
  start() {
    const config = this.core.config;
    if (!config?.wallpapers?.length) return;

    this.stop(); // 先停止现有的

    const interval = (config.wallpaperInterval || 30) * 1000;
    this.changeWallpaper(); // 立即显示第一张

    this.interval = setInterval(() => {
      this.changeWallpaper();
    }, interval);
  }

  // 停止壁纸轮播
  stop() {
    if (this.interval) {
      clearInterval(this.interval);
      this.interval = null;
    }
  }

  // 更换壁纸 - 消除重复DOM操作
  changeWallpaper() {
    const config = this.core.config;
    if (!config?.wallpapers?.length) return;

    const wallpaper = document.getElementById('wallpaper');
    if (!wallpaper) return;

    wallpaper.style.backgroundImage = `url('${config.wallpapers[this.currentIndex]}')`;
    this.currentIndex = (this.currentIndex + 1) % config.wallpapers.length;
  }
}

// 事件管理器 - 消除重复事件渲染
class EventManager {
  constructor(core) {
    this.core = core;
  }

  // 获取即将发生的事件 - 消除重复过滤逻辑
  getUpcomingEvents(limit = 3) {
    const config = this.core.config;
    if (!config?.events?.length) return [];

    const now = new Date();
    
    return config.events
      .filter(event => new Date(event.date) >= now)
      .sort((a, b) => new Date(a.date) - new Date(b.date))
      .slice(0, limit);
  }

  // 格式化事件显示 - 消除重复格式化代码
  formatEvent(event) {
    const eventDate = new Date(event.date);
    const now = new Date();
    const daysUntil = ClassPaperCore.calculateDaysBetween(now, eventDate);
    const dateStr = ClassPaperCore.formatDate(eventDate, { month: 'short', day: 'numeric' });

    return {
      name: event.name,
      date: dateStr,
      daysUntil: daysUntil
    };
  }
}

// 进度管理器 - 消除重复进度计算
class ProgressManager {
  constructor(core) {
    this.core = core;
  }

  // 计算学期进度 - 消除重复进度逻辑
  calculateProgress() {
    const config = this.core.config;
    if (!config?.lessons?.times?.semester) return null;

    const semester = config.lessons.times.semester;
    const now = new Date();
    const begin = new Date(semester.begin);
    const end = new Date(semester.end);

    if (now < begin) return { percent: 0, remaining: ClassPaperCore.calculateDaysBetween(now, end) };
    if (now > end) return { percent: 100, remaining: 0 };

    const totalDays = ClassPaperCore.calculateDaysBetween(begin, end);
    const passedDays = ClassPaperCore.calculateDaysBetween(begin, now);
    const remainingDays = totalDays - passedDays;
    const percent = Math.max(0, Math.min(100, (passedDays / totalDays) * 100));

    return {
      percent: Math.round(percent),
      remaining: remainingDays,
      description: config.progressDescription || '学习进度'
    };
  }
}

// 导出统一的API
window.ClassPaper = {
  Core: ClassPaperCore,
  ScheduleManager,
  WallpaperManager,
  EventManager,
  ProgressManager
};