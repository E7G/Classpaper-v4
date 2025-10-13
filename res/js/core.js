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
    this.monetColors = null;
    this.colorCache = new Map(); // 莫奈颜色缓存
    this.preloadedWallpapers = []; // 预加载壁纸列表
    this.maxCacheSize = 50; // 缓存大小限制
    this.recentWallpapers = []; // 最近使用的壁纸（用于智能预测）
    this.cacheHits = 0; // 缓存命中次数
    this.cacheMisses = 0; // 缓存未命中次数
  }

  // 提取莫奈配色 - 带缓存（按需加载版本）
  async extractMonetColors(wallpaperPath) {
    // 检查缓存
    if (this.colorCache.has(wallpaperPath)) {
      console.log(`[Cache] 命中缓存: ${wallpaperPath}`);
      this.cacheHits++; // 统计缓存命中
      const cachedResult = this.colorCache.get(wallpaperPath);
      this.monetColors = cachedResult.colors;
      
      if (cachedResult.css) {
        this.injectMonetCSS(cachedResult.css);
      }
      
      this.core.emit('monet:colorsExtracted', cachedResult);
      return cachedResult;
    }
    
    this.cacheMisses++; // 统计缓存未命中

    // 检查后端API是否可用
    if (typeof window.extractMonetColors !== 'function') {
      console.warn('[Monet] 后端API未就绪，使用默认配色');
      const defaultResult = {
        success: false,
        error: '后端API未就绪',
        colors: this.getDefaultMonetColors(),
        css: '',
        isDark: false
      };
      this.monetColors = defaultResult.colors;
      this.core.emit('monet:colorsExtracted', defaultResult);
      return defaultResult;
    }

    // 缓存未命中，立即提取
    try {
      console.log(`[Monet] 缓存未命中，开始取色: ${wallpaperPath}`);
      const result = await window.extractMonetColors(wallpaperPath);
      
      if (result.success) {
        this.monetColors = result.colors;
        console.log(`[Monet] 取色成功，主色调: ${result.colors.primary}`);
        
        // 缓存结果
        this.colorCache.set(wallpaperPath, result);
        
        // 缓存大小管理 - 避免内存泄漏
        if (this.colorCache.size > this.maxCacheSize) {
          const firstKey = this.colorCache.keys().next().value;
          this.colorCache.delete(firstKey);
          console.log(`[Monet] 清理缓存，当前大小: ${this.colorCache.size}`);
        }
        
        // 注入CSS变量到页面
        if (result.css) {
          this.injectMonetCSS(result.css);
        }
        
        // 触配色更新事件
        this.core.emit('monet:colorsExtracted', result);
        
        return result;
      } else {
        console.warn(`[Monet] 取色失败，使用默认配色: ${result.error}`);
        this.monetColors = result.colors;
        return result;
      }
    } catch (error) {
      console.error('[Monet] 取色过程出错:', error);
      return {
        success: false,
        error: error.message,
        colors: this.getDefaultMonetColors()
      };
    }
  }

  // 注入莫奈配色CSS变量
  injectMonetCSS(css) {
    // 移除旧的莫奈样式
    const oldStyle = document.getElementById('monet-dynamic-styles');
    if (oldStyle) {
      oldStyle.remove();
    }
    
    // 创建新的样式元素
    const style = document.createElement('style');
    style.id = 'monet-dynamic-styles';
    style.textContent = css;
    document.head.appendChild(style);
    
    console.log('[Monet] CSS变量已注入到页面');
  }

  // 获取默认莫奈配色（当取色失败时使用）
  getDefaultMonetColors() {
    return {
      primary: "#667eea",
      primaryVariant: "#764ba2",
      secondary: "#f093fb",
      secondaryVariant: "#f5576c",
      background: "#ffffff",
      surface: "#fafafa",
      error: "#f44336",
      onPrimary: "#ffffff",
      onSecondary: "#ffffff",
      onBackground: "#1a1a1a",
      onSurface: "#1a1a1a",
      onError: "#ffffff",
    };
  }

  // 开始壁纸轮播 - 智能预加载
  async start() {
    const config = this.core.config;
    if (!config?.wallpapers?.length) return;

    this.stop(); // 先停止现有的

    // 只预加载第一张壁纸，其余采用按需+后台预加载
    await this.preloadFirstWallpaper();

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

  // 智能预加载 - 只预加载第一张壁纸（零延迟启动）
  async preloadFirstWallpaper() {
    const config = this.core.config;
    if (!config?.wallpapers?.length) return;

    // 打乱壁纸顺序，实现随机效果
    this.preloadedWallpapers = [...config.wallpapers].sort(() => Math.random() - 0.5);
    this.currentIndex = 0;

    // 只预加载第一张壁纸 - 使用私有方法统一处理
    const firstWallpaper = this.preloadedWallpapers[0];
    if (firstWallpaper) {
      // 立即开始预加载，但不阻塞UI
      this.preloadSingleWallpaper(firstWallpaper);
    }
  }

  // 智能预加载管理器 - 实现真正"无感"体验
  async preloadManager() {
    if (this.preloadedWallpapers.length <= 1) return;

    // 获取未缓存的壁纸列表
    const uncachedWallpapers = this.preloadedWallpapers.filter(wp => !this.colorCache.has(wp));
    if (uncachedWallpapers.length === 0) return;

    // 智能预测：优先预加载"最近使用"的壁纸
    const recentWallpapers = this.getRecentWallpapers(5); // 获取最近5次使用的壁纸
    const priorityWallpapers = uncachedWallpapers.filter(wp => recentWallpapers.includes(wp));
    const normalWallpapers = uncachedWallpapers.filter(wp => !recentWallpapers.includes(wp));

    // 动态调整预加载数量（基于缓存命中率）
    const hitRate = this.calculateHitRate();
    const preloadCount = Math.min(hitRate < 0.7 ? 4 : 2, uncachedWallpapers.length); // 命中率低时多预加载
    const wallpapersToPreload = [];
    
    // 优先预加载最近使用的壁纸
    for (let i = 0; i < Math.min(priorityWallpapers.length, Math.ceil(preloadCount * 0.6)); i++) {
      wallpapersToPreload.push(priorityWallpapers[i]);
    }
    
    // 补充随机选择的壁纸
    const remainingCount = preloadCount - wallpapersToPreload.length;
    for (let i = 0; i < remainingCount && normalWallpapers.length > 0; i++) {
      const randomIndex = Math.floor(Math.random() * normalWallpapers.length);
      const selectedWallpaper = normalWallpapers.splice(randomIndex, 1)[0];
      if (selectedWallpaper) {
        wallpapersToPreload.push(selectedWallpaper);
      }
    }

    // 并行预加载（真正的无感体验）
    const preloadPromises = wallpapersToPreload.map(wallpaper => 
      this.preloadSingleWallpaper(wallpaper)
    );

    // 不等待结果，完全后台执行
    Promise.allSettled(preloadPromises);
  }

  // 单张壁纸预加载（私有方法）
  async preloadSingleWallpaper(wallpaper) {
    // 检查后端API是否可用
    if (typeof window.extractMonetColors !== 'function') {
      console.warn(`[Preload] 后端API未就绪，跳过预加载: ${wallpaper}`);
      return;
    }

    try {
      console.log(`[Preload] 智能预加载: ${wallpaper}`);
      const result = await window.extractMonetColors(wallpaper);
      if (result.success) {
        this.colorCache.set(wallpaper, result);
        console.log(`[Preload] 智能预加载成功: ${wallpaper}`);
      }
    } catch (error) {
      console.warn(`[Preload] 智能预加载失败: ${wallpaper}`, error);
    }
  }

  // 更新最近使用的壁纸列表（智能预测用）
  updateRecentWallpapers(wallpaper) {
    // 移除已存在的相同壁纸
    this.recentWallpapers = this.recentWallpapers.filter(wp => wp !== wallpaper);
    // 添加到开头
    this.recentWallpapers.unshift(wallpaper);
    // 保持最多10个记录
    if (this.recentWallpapers.length > 10) {
      this.recentWallpapers = this.recentWallpapers.slice(0, 10);
    }
  }

  // 获取最近使用的壁纸（智能预测用）
  getRecentWallpapers(count) {
    return this.recentWallpapers.slice(0, count);
  }

  // 计算缓存命中率（智能预测用）
  calculateHitRate() {
    const total = this.cacheHits + this.cacheMisses;
    return total > 0 ? this.cacheHits / total : 0;
  }

  // 随机选择下一张壁纸
  getNextRandomWallpaper() {
    if (this.preloadedWallpapers.length === 0) return null;
    
    // 随机选择索引
    const randomIndex = Math.floor(Math.random() * this.preloadedWallpapers.length);
    this.currentIndex = randomIndex;
    
    return this.preloadedWallpapers[randomIndex];
  }

  // 更换壁纸 - 集成莫奈取色（真正无感版本）
  changeWallpaper() {
    const config = this.core.config;
    if (!config?.wallpapers?.length) return;

    const wallpaper = document.getElementById('wallpaper');
    if (!wallpaper) return;

    // 使用预加载列表或原始列表
    const wallpaperList = this.preloadedWallpapers.length > 0 ? this.preloadedWallpapers : config.wallpapers;
    const currentWallpaper = this.getNextRandomWallpaper() || wallpaperList[0];
    
    wallpaper.style.backgroundImage = `url('${currentWallpaper}')`;
    
    // 记录最近使用的壁纸（用于智能预测）
    this.updateRecentWallpapers(currentWallpaper);
    
    // 触发壁纸变化事件，让主界面可以提取莫奈配色
    this.core.emit('wallpaper:changed', currentWallpaper);
    
    // 立即触发智能预加载（不等待，完全后台）
    // 使用setTimeout确保当前流程立即完成
    setTimeout(() => this.preloadManager(), 0);
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