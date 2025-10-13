// ClassPaper 主界面 - 使用核心库重构
class MainInterface {
  constructor() {
    this.core = new ClassPaper.Core();
    this.scheduleManager = new ClassPaper.ScheduleManager(this.core);
    this.wallpaperManager = new ClassPaper.WallpaperManager(this.core);
    this.eventManager = new ClassPaper.EventManager(this.core);
    this.progressManager = new ClassPaper.ProgressManager(this.core);
    
    this.init();
  }

  async init() {
    try {
      await this.core.loadConfig();
      this.setupEventListeners();
      this.render();
      this.startPeriodicUpdates();
    } catch (error) {
      console.error('主界面初始化失败:', error);
      this.showError('初始化失败，请检查配置');
    }
  }

  setupEventListeners() {
    // 配置加载完成事件
    this.core.on('config:loaded', () => {
      this.render();
    });

    // 设置按钮事件
    const settingsBtn = document.getElementById('settingsBtn');
    if (settingsBtn) {
      settingsBtn.addEventListener('click', () => this.openSettings());
    }
  }

  render() {
    this.updateDate();
    this.renderSchedule();
    this.renderProgress();
    this.renderEvents();
    this.renderNote();
    // 异步启动壁纸系统
    this.startWallpaper().catch(error => {
      console.error('壁纸系统启动失败:', error);
    });
  }

  // 更新日期显示 - 靠右侧显示并添加周数
  updateDate() {
    const dateElement = document.getElementById('currentDate');
    const clockElement = document.getElementById('clockTime');
    if (!dateElement) return;

    const now = new Date();
    // 计算当前是第几周（基于学期开始时间）
    const weekNumber = this.calculateWeekNumber(now);
    
    // 靠右侧显示的格式：第X周 10.12 周一
    const month = now.getMonth() + 1;
    const day = now.getDate();
    const dayName = ClassPaper.Core.getDayName(now);
    const dateStr = `第${weekNumber}周 ${month}.${day.toString().padStart(2, '0')} ${dayName}`;
    dateElement.textContent = dateStr;
    
    // 设置靠右对齐样式
    dateElement.style.textAlign = 'right';
    dateElement.style.marginRight = '1rem';
    
    // 更新时钟显示 - 机械转轮动画
    if (clockElement) {
      const timeStr = now.toLocaleTimeString('zh-CN', {
        hour12: false,
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit'
      });
      
      // 使用机械转轮动画更新时钟
      this.updateClockWithAnimation(clockElement, timeStr);
    }
  }

  // 机械转轮时钟动画 - 翻页效果
    updateClockWithAnimation(clockElement, newTime) {
        const oldTime = clockElement.textContent;
        if (oldTime === newTime) return;

        // 真正的翻页效果：数字从上方滑入，旧数字向下方滑出
        const oldDigits = oldTime.replace(/:/g, '').split('');
        const newDigits = newTime.replace(/:/g, '').split('');
        
        // 构建翻页数字HTML
        let html = '';
        for (let i = 0; i < 6; i++) {
            const oldDigit = oldDigits[i] || '0';
            const newDigit = newDigits[i];
            const shouldAnimate = oldDigit !== newDigit;
            
            if (shouldAnimate) {
                // 变化的数字：创建翻页效果
                html += `<span class="flip-digit">
                    <span class="digit-old">${oldDigit}</span>
                    <span class="digit-new">${newDigit}</span>
                </span>`;
            } else {
                // 不变的数字：直接显示
                html += newDigit;
            }
            
            // 添加冒号分隔符
            if (i === 1 || i === 3) {
                html += ':';
            }
        }

        clockElement.innerHTML = html;
        
        // 动画结束后清理DOM，但保持显示新时间
        setTimeout(() => {
            // 将所有翻页数字替换为普通数字，保持时间显示
            let finalHtml = '';
            for (let i = 0; i < 6; i++) {
                finalHtml += newDigits[i];
                if (i === 1 || i === 3) {
                    finalHtml += ':';
                }
            }
            clockElement.innerHTML = finalHtml;
        }, 600);
    }

  // 渲染课程表 - 优化信息密度和视觉层次
  renderSchedule() {
    const grid = document.getElementById('scheduleGrid');
    const header = document.getElementById('scheduleHeader');
    const body = document.getElementById('scheduleBody');
    
    if (!grid || !header || !body || !this.core.config?.lessons) return;

    const relevantCourses = this.scheduleManager.getTodayRelevantCourses();
    
    // 优化卡片布局：减少冗余信息，突出关键内容
    grid.innerHTML = relevantCourses.map(course => {
      const statusClass = course.isCurrent ? 'current' : course.isNext ? 'next' : course.isPast ? 'past' : '';
      const statusText = course.isCurrent ? '进行中' : course.isNext ? '即将开始' : '';
      
      return `
        <div class="course-card ${statusClass}">
          <div class="course-period">${course.period}</div>
          <div class="course-name">${course.className}</div>
          <div class="course-time">${course.time.begin}-${course.time.end}</div>
          ${statusText ? `
            <div class="course-status">
              <div class="status-indicator"></div>
              <span>${statusText}</span>
            </div>
          ` : ''}
        </div>
      `;
    }).join('');
    
    // 保留原表格作为备选（隐藏状态）
    header.innerHTML = '<tr><th>时间</th><th>课程</th><th>状态</th></tr>';
    body.innerHTML = relevantCourses.map(course => {
      const statusClass = course.isCurrent ? 'current-period' : course.isNext ? 'next-period' : '';
      const statusText = course.isCurrent ? '当前' : course.isNext ? '下一节' : '';
      
      return `
        <tr class="${statusClass}">
          <td><strong>${course.period}</strong><br><small>${course.time.begin}-${course.time.end}</small></td>
          <td>${course.className}</td>
          <td>${statusText}</td>
        </tr>
      `;
    }).join('');
  }

  // 高亮当前课程 - 消除重复高亮逻辑
  highlightCurrentPeriod() {
    // 清除之前的高亮
    document.querySelectorAll('.current-period').forEach(el => {
      el.classList.remove('current-period');
    });

    const currentClass = this.scheduleManager.getCurrentClass();
    if (!currentClass) return;

    const todayName = ClassPaper.Core.getDayName(new Date());
    const cell = document.querySelector(`td[data-day="${todayName}"][data-period="${currentClass.period}"]`);
    
    if (cell) {
      cell.classList.add('current-period');
    }
  }

  // 渲染进度 - 真实反映时间进度
  renderProgress() {
    const progressTitle = document.getElementById('progressTitle');
    const progressDays = document.getElementById('progressDays');
    const progressFill = document.getElementById('progressFill');
    
    if (!progressTitle || !progressDays || !progressFill) return;

    const progress = this.progressManager.calculateProgress();
    if (!progress) return;

    // 同步文本与进度条：显示真实的时间进度
    progressTitle.textContent = progress.description;
    progressDays.textContent = `${progress.percent}%`;
    progressFill.style.width = `${progress.percent}%`;
  }

  // 渲染事件 - 倒计时小组件重构
  renderEvents() {
    const eventsList = document.getElementById('eventsList');
    if (!eventsList) return;

    const upcomingEvents = this.eventManager.getUpcomingEvents();
    
    if (upcomingEvents.length === 0) {
      eventsList.innerHTML = '<p style="color: #666; text-align: center;">暂无即将发生的事件</p>';
      return;
    }

    eventsList.innerHTML = upcomingEvents.map(event => {
      const formatted = this.eventManager.formatEvent(event);
      return `
        <div class="event-item">
          <div class="countdown-container">
            <div class="countdown-number">${formatted.daysUntil}</div>
            <div class="countdown-unit">天</div>
          </div>
          <div class="event-title">${formatted.name}</div>
        </div>
      `;
    }).join('');
  }

  // 渲染备注 - 消除重复备注逻辑
  renderNote() {
    const noteElement = document.getElementById('note');
    if (!noteElement || !this.core.config?.note) return;

    noteElement.textContent = this.core.config.note;
  }

  // 计算周数 - 基于学期开始时间
  calculateWeekNumber(date) {
    const config = this.core.config;
    if (!config?.lessons?.times?.semester?.start) {
      // 如果没有配置学期开始时间，使用默认计算（9月1日为第1周）
      const year = date.getFullYear();
      const semesterStart = new Date(year, 8, 1); // 9月1日
      
      // 如果当前日期在9月1日之前，使用前一年的9月1日
      if (date < semesterStart) {
        semesterStart.setFullYear(year - 1);
      }
      
      const weeksDiff = Math.ceil((date - semesterStart) / (7 * 24 * 60 * 60 * 1000));
      return Math.max(1, weeksDiff);
    }
    
    // 使用配置的学期开始时间
    const semesterStart = new Date(config.lessons.times.semester.start);
    const weeksDiff = Math.ceil((date - semesterStart) / (7 * 24 * 60 * 60 * 1000));
    return Math.max(1, weeksDiff + 1); // +1因为第0周应该显示为第1周
  }

  // 启动壁纸 - 集成莫奈取色（异步）
  async startWallpaper() {
    await this.wallpaperManager.start();
    
    // 监听壁纸变化事件，自动提取莫奈配色
    this.wallpaperManager.core.on('wallpaper:changed', async (wallpaperPath) => {
      console.log(`[Main] 壁纸已切换，开始莫奈取色: ${wallpaperPath}`);
      await this.wallpaperManager.extractMonetColors(wallpaperPath);
    });
    
    // 监听配色提取完成事件
    this.wallpaperManager.core.on('monet:colorsExtracted', (result) => {
      if (result.success) {
        console.log(`[Main] 莫奈配色已应用，主色调: ${result.colors.primary}`);
        
        // 根据明暗模式调整UI
        if (result.isDark) {
          document.body.classList.add('monet-dark-mode');
          document.body.classList.remove('monet-light-mode');
        } else {
          document.body.classList.add('monet-light-mode');
          document.body.classList.remove('monet-dark-mode');
        }
      }
    });
  }

  // 定期更新 - 消除重复定时器
  startPeriodicUpdates() {
    // 每秒更新时钟和日期
    setInterval(() => this.updateDate(), 1000);
    
    // 每秒检查当前课程
    setInterval(() => this.highlightCurrentPeriod(), 1000);
  }

  // 打开设置 - 消除重复窗口逻辑
  openSettings() {
    // 如果在前端环境中运行，直接导航
    if (window.location.protocol === 'file:' || window.location.hostname === 'localhost') {
      window.location.href = 'settings.html';
    } else {
      // 在应用中运行，调用后端接口
      if (window.external && window.external.openSettings) {
        window.external.openSettings();
      } else {
        // 降级处理：打开新窗口
        window.open('settings.html', '_blank');
      }
    }
  }

  // 错误处理 - 消除重复错误显示
  showError(message) {
    const errorDiv = document.createElement('div');
    errorDiv.className = 'error-message';
    errorDiv.innerHTML = `
      <div class="glass-card" style="background: rgba(255, 107, 107, 0.9);">
        <h3>错误</h3>
        <p>${message}</p>
        <button class="btn btn-secondary" onclick="this.parentElement.parentElement.remove()">关闭</button>
      </div>
    `;
    document.body.appendChild(errorDiv);
  }
}

// 初始化主界面
document.addEventListener('DOMContentLoaded', async () => {
  const mainInterface = new MainInterface();
  // 等待壁纸系统初始化完成
  if (mainInterface.wallpaperManager) {
    console.log('[Main] 等待壁纸预处理完成...');
  }
});