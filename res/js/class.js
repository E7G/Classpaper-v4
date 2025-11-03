/* 课表优化版 */

const source = lessons;
const source_vec = source.split(',');

const classtable = document.getElementById('classtable');

// 音频元素
const regularNotification = document.getElementById('regularNotification');
const endingNotification = document.getElementById('endingNotification');

// 上次提示的时间戳
let lastNotificationTime = 0;
// 是否已经发出结束提示
let endingNotified = false;

// 存储定时器ID，用于暂停和恢复
let refreshIntervalId = null;
// 是否处于调试模式
let isDebugMode = false;

// 播放提示音
function playNotification(type, className) {
    if (!CONFIG.notifications.enabled) return;
    if (!className || className === '无') return; // 课程为无时不响铃
    if (type === 'regular') {
        regularNotification && regularNotification.play();
    } else if (type === 'ending') {
        endingNotification && endingNotification.play();
    }
}

// 获取今天、前一天、后一天的课程数组，并带有详细注释
function getDayVectors() {
    // 获取当前星期几，getDay()返回0-6，0表示周日
    let week = new Date().getDay();
    // 如果是周日（0），则将其转换为7，方便后续计算（1=周一，7=周日）
    week = week === 0 ? 7 : week;

    // 直接从CONFIG中获取课程数据，而不是从source_vec解析
    const dayIndex = week - 1; // 转换为0-6的索引，0=周一
    const todayClasses = CONFIG.lessons.schedule[dayIndex]?.classes || [];
    
    // 获取前一天的课程
    const prevDayIndex = dayIndex === 0 ? 6 : dayIndex - 1;
    const prevClasses = CONFIG.lessons.schedule[prevDayIndex]?.classes || [];
    
    // 获取后一天的课程
    const nextDayIndex = dayIndex === 6 ? 0 : dayIndex + 1;
    const nextClasses = CONFIG.lessons.schedule[nextDayIndex]?.classes || [];
    
    // 返回今天、前一天、后一天的课程数组
    return {
        today_vec: [...todayClasses, ""], // 今天的课程，末尾加一个空元素
        prev_vec: [...prevClasses, ""],   // 前一天的课程，末尾加一个空元素
        next_vec: [...nextClasses, ""]    // 后一天的课程，末尾加一个空元素
    };
}

// 重新排列课程顺序，当前课程在第7个位置
function arrangeClasses(currentIndex, today_vec, prev_vec, next_vec) {
    // console.log('[arrangeClasses] ===== 开始排列课程 =====');
    // console.log(`[arrangeClasses] 当前课程索引: ${currentIndex}`);
    // console.log('[arrangeClasses] 今日课程向量:', today_vec);
    // console.log('[arrangeClasses] 昨日课程向量:', prev_vec);
    // console.log('[arrangeClasses] 明日课程向量:', next_vec);
    
    // showBefore 表示当前课程前面要显示的课程数，这里为6
    const showBefore = 6;
    // classes 是今天的课程数组，去掉最后一个元素（通常为占位或空元素）
    const classes = today_vec.slice(0, -1);
    // console.log('[arrangeClasses] 今日有效课程:', classes);
    
    // arranged 用于存放最终排列好的12节课
    const arranged = new Array(12);
    // console.log('[arrangeClasses] 初始化12个位置的数组');

    // 填充前6节课
    // console.log('[arrangeClasses] ===== 开始填充前6节课 =====');
    for (let i = 0; i < showBefore; i++) {
        // idx 计算当前要填充的课程在 today_vec 中的下标
        let idx = currentIndex - (showBefore - i);
        // console.log(`[arrangeClasses] 位置[${i}] 计算索引: ${idx}`);
        
        if (idx >= 0) {
            // 如果 idx 合法，直接取今天的课程
            arranged[i] = classes[idx];
            // console.log(`[arrangeClasses] 位置[${i}] 取今日课程: ${arranged[i]}`);
        } else if (prev_vec) {
            // 如果 idx 不合法，尝试从前一天的课程补齐
            let prevIndex = prev_vec.length - 1 + idx;
            // console.log(`[arrangeClasses] 位置[${i}] 计算昨日索引: ${prevIndex}`);
            if (prevIndex >= 0) {
                arranged[i] = prev_vec[prevIndex];
                // console.log(`[arrangeClasses] 位置[${i}] 取昨日课程: ${arranged[i]}`);
            } else {
                arranged[i] = "";
                // console.log(`[arrangeClasses] 位置[${i}] 昨日索引无效，设为空`);
            }
        } else {
            // 没有前一天的课程，填空
            arranged[i] = "";
            // console.log(`[arrangeClasses] 位置[${i}] 无昨日数据，设为空`);
        }
    }

    // 填充当前课程
    arranged[showBefore] = classes[currentIndex];
    // console.log(`[arrangeClasses] 当前课程位置[${showBefore}]: ${arranged[showBefore]}`);

    // 填充后5节课
    // console.log('[arrangeClasses] ===== 开始填充后5节课 =====');
    for (let i = showBefore + 1; i < 12; i++) {
        // idx 计算当前要填充的课程在 today_vec 中的下标
        let idx = currentIndex + (i - showBefore);
        // console.log(`[arrangeClasses] 位置[${i}] 计算索引: ${idx}`);
        
        if (idx < classes.length) {
            // 如果 idx 合法，直接取今天的课程
            arranged[i] = classes[idx];
            // console.log(`[arrangeClasses] 位置[${i}] 取今日课程: ${arranged[i]}`);
        } else if (next_vec) {
            // 如果 idx 超出 today_vec，尝试从后一天的课程补齐
            let nextIndex = idx - classes.length;
            // console.log(`[arrangeClasses] 位置[${i}] 计算明日索引: ${nextIndex}`);
            if (nextIndex < next_vec.length - 1) {
                arranged[i] = next_vec[nextIndex];
                // console.log(`[arrangeClasses] 位置[${i}] 取明日课程: ${arranged[i]}`);
            } else {
                arranged[i] = "";
                // console.log(`[arrangeClasses] 位置[${i}] 明日索引无效，设为空`);
            }
        } else {
            // 没有后一天的课程，填空
            arranged[i] = "";
            // console.log(`[arrangeClasses] 位置[${i}] 无明日数据，设为空`);
        }
    }

    // console.log('[arrangeClasses] 最终排列结果:', arranged);
    // console.log('[arrangeClasses] ===== 课程排列完成 =====\n');
    return arranged;
}

// 解析时间字符串为当天的Date对象
function parseTime(timeStr) {
    const [hours, minutes] = timeStr.split(':').map(Number);
    const now = new Date();
    now.setHours(hours, minutes, 0, 0);
    return now.getTime();
}

// 查找最近的上一节课和下一节课
function findNearestClasses(now, schedule) {
    // console.log('[findNearestClasses] ===== 开始查找最近课程 =====');
    // console.log(`[findNearestClasses] 当前时间戳: ${now}`);
    // console.log('[findNearestClasses] 当前时间:', new Date(now).toLocaleTimeString());
    // console.log('[findNearestClasses] 课程时间表:', schedule);
    
    let prevIdx = -1, nextIdx = -1;
    let prevTime = -Infinity, nextTime = Infinity;
    
    // console.log('[findNearestClasses] ===== 遍历课程查找最近时间 =====');
    for (let i = 0; i < schedule.length; i++) {
        const classBegin = parseTime(schedule[i].begin);
        const classEnd = parseTime(schedule[i].end);
        // console.log(`[findNearestClasses] 第${i}节课: ${schedule[i].begin}-${schedule[i].end}`);
        // console.log(`[findNearestClasses] 时间范围: ${new Date(classBegin).toLocaleTimeString()}-${new Date(classEnd).toLocaleTimeString()}`);
        
        if (classEnd <= now && classEnd > prevTime) {
            prevTime = classEnd;
            prevIdx = i;
            // console.log(`[findNearestClasses] 更新上一节课: 索引=${prevIdx}, 结束时间=${new Date(prevTime).toLocaleTimeString()}`);
        }
        if (classBegin > now && classBegin < nextTime) {
            nextTime = classBegin;
            nextIdx = i;
            // console.log(`[findNearestClasses] 更新下一节课: 索引=${nextIdx}, 开始时间=${new Date(nextTime).toLocaleTimeString()}`);
        }
    }
    
    // console.log('[findNearestClasses] ===== 查找完成 =====');
    // console.log(`[findNearestClasses] 结果 - 上一节课索引: ${prevIdx}, 下一节课索引: ${nextIdx}`);
    // if (prevIdx !== -1) {
    //     console.log(`[findNearestClasses] 上一节课: ${schedule[prevIdx].begin}-${schedule[prevIdx].end}`);
    // }
    // if (nextIdx !== -1) {
    //     console.log(`[findNearestClasses] 下一节课: ${schedule[nextIdx].begin}-${schedule[nextIdx].end}`);
    // }
    // console.log('[findNearestClasses] ===== 结束查找 =====\n');
    
    return { prevIdx, nextIdx };
}

// 新增判断当前是否有课的函数
// function hasCurrentClass(now, schedule) {
//     for (const period of schedule) {
//         const classBegin = parseTime(period.begin);
//         const classEnd = parseTime(period.end);
//         if (now >= classBegin && now <= classEnd) {
//             return true;
//         }
//     }
//     return false;
// }

// 主函数：刷新当前课程显示
function nowClass() {
    // console.log('[nowClass] ===== 开始刷新课程显示 =====');
    const date = new Date();
    // console.log('[nowClass] 当前时间:', date.toLocaleString());
    
    const { today_vec, prev_vec, next_vec } = getDayVectors();
    // console.log('[nowClass] 今日课程向量:', today_vec);
    // console.log('[nowClass] 昨日课程向量:', prev_vec);
    // console.log('[nowClass] 明日课程向量:', next_vec);
    
    const displayMode = CONFIG.lessons.displayMode || 'scroll';
    // console.log('[nowClass] 显示模式:', displayMode);

    // 检查是否在学期时间范围
    const semesterBegin = new Date(CONFIG.lessons.times.semester.begin);
    const semesterEnd = new Date(CONFIG.lessons.times.semester.end);
    // console.log('[nowClass] 学期开始:', semesterBegin.toLocaleDateString());
    // console.log('[nowClass] 学期结束:', semesterEnd.toLocaleDateString());
    // console.log('[nowClass] 当前是否在学期内:', date >= semesterBegin && date <= semesterEnd);
    
    if (date < semesterBegin || date > semesterEnd) {
        // console.log('[nowClass] 不在学期内，显示"无"');
        for (let i = 0; i < 12; i++) {
            let opacity = (i === 6) ? "" : "opacity: 0.5;";
            document.getElementById('c' + i).innerHTML =
                `<a href="#" role="button" class="contrast" id="c_b${i}" style="${opacity}">无</a>`;
        }
        const c_b6 = document.getElementById('c_b6');
        c_b6.style.backgroundColor = '#93cee97f';
        c_b6.style.fontWeight = '600';
        c_b6.style.opacity = '1';
        for (let i = 0; i < 6; i++) {
            const el = document.getElementById('c_b' + i);
            el.style.backgroundColor = '#3daee940';
            el.style.fontWeight = '400';
        }
        for (let i = 7; i < 12; i++) {
            const el = document.getElementById('c_b' + i);
            el.style.backgroundColor = '';
            el.style.fontWeight = '400';
        }
        // console.log('[nowClass] ===== 学期外显示完成 =====');
        return;
    }

    const schedule = CONFIG.lessons.times.schedule;
    // console.log('[nowClass] 课程时间表:', schedule);
    const now = date.getTime();
    // console.log('[nowClass] 当前时间戳:', now);
    let it = -1;
    let colorize = true;
    let inClassTime = false;
    let inRestTime = false;

    // console.log('[nowClass] ===== 开始检查课程状态 =====');
    for (let i = 0; i < schedule.length; i++) {
        const period = schedule[i];
        const classBegin = parseTime(period.begin);
        const classEnd = parseTime(period.end);
        // console.log(`[nowClass] 检查第${i}节课: ${period.begin}-${period.end}, 时间范围: ${new Date(classBegin).toLocaleTimeString()}-${new Date(classEnd).toLocaleTimeString()}`);
        
        if (now >= classBegin && now <= classEnd) {
            it = i;
            inClassTime = true;
            const remainingTime = (classEnd - now) / (1000 * 60);
            const regularInterval = CONFIG.notifications.regularInterval;
            // console.log(`[nowClass] 当前在第${i}节课内，剩余时间: ${remainingTime.toFixed(1)}分钟`);
            
            // 获取当前课程名
            let className = '';
            if (displayMode === 'day') {
                const { today_vec } = getDayVectors();
                className = today_vec[i] || '';
            } else {
                const { today_vec } = getDayVectors();
                className = today_vec[i] || '';
            }
            // console.log(`[nowClass] 当前课程名称: ${className}`);
            
            if (remainingTime > CONFIG.notifications.endingTime &&
                (now - lastNotificationTime) >= regularInterval * 60 * 1000) {
                // console.log(`[nowClass] 发送常规提醒，间隔: ${regularInterval}分钟`);
                playNotification('regular', className);
                lastNotificationTime = now;
            }
            if (remainingTime <= CONFIG.notifications.endingTime && !endingNotified) {
                // console.log(`[nowClass] 发送下课提醒，阈值: ${CONFIG.notifications.endingTime}分钟`);
                playNotification('ending', className);
                endingNotified = true;
            }
            break;
        } else if (period.rest && i > 0) {
            const restBegin = schedule[i - 1].end;
            const restTime = parseTime(restBegin);
            if (now >= classEnd && now <= restTime) {
                it = i;
                colorize = false;
                endingNotified = false;
                inRestTime = true;
                // console.log(`[nowClass] 当前在第${i-1}节课和第${i}节课之间的休息时间`);
                break;
            }
        }
    }
    // console.log(`[nowClass] 检查完成 - 课程索引: ${it}, 是否在上课时间: ${inClassTime}, 是否在休息时间: ${inRestTime}`);

    if (displayMode === 'scroll') {
        // 滚动模式（新逻辑：智能填充课程）
        // console.log('[nowClass] ===== 开始滚动模式处理 =====');
        
        // 修复：优先使用实际当前课程索引，而不是最近的课程
        let currentIndex;
        if (inClassTime && it !== -1) {
            // 如果当前在上课时间，直接使用当前课程索引
            currentIndex = it;
            // console.log(`[nowClass] 使用当前实际课程索引: ${currentIndex}`);
        } else {
            // 不在上课时间时，才使用最近的课程
            const { prevIdx, nextIdx } = findNearestClasses(now, schedule);
            currentIndex = nextIdx !== -1 ? nextIdx : prevIdx;
            // console.log(`[nowClass] 使用最近课程索引: ${currentIndex} (上一节: ${prevIdx}, 下一节: ${nextIdx})`);
        }
        
        let arranged = arrangeClasses(currentIndex, today_vec, prev_vec, next_vec);
        // console.log('[nowClass] 初始排列课程:', arranged);

        // 修复：只有当确实不在上课时间时才显示休息
        if (!inClassTime) {
            // console.log('[nowclass] 无当前课程，开始插入休息并重组课程');
            let newArranged = [...arranged];
            
            // 计算中间位置，根据实际课程长度自适应
            const middleIndex = Math.floor(newArranged.length / 2);
            
            // 在中间位置插入休息，但确保不吞掉课程
            newArranged.splice(middleIndex, 0, "休息");
            
            arranged = newArranged;
            // console.log('[nowclass] 重组完成后的课程排列:', arranged); 
        } else {
            // console.log('[nowclass] 当前在上课时间，不显示休息，直接使用实际课程');
        }

        // 计算实际显示的课程数量
        const displayCount = arranged.length;
        const middleIndex = Math.floor(displayCount / 2);
        
        // 渲染课程表
        // console.log('[nowClass] ===== 开始渲染课程表 =====');
        for (let i = 0; i < displayCount; i++) {
            let content = arranged[i] || "";
            let opacity = (i === middleIndex) ? "" : "opacity: 0.5;";
            // console.log(`[nowClass] 渲染课程[${i}]: ${content}, 样式: ${opacity || '默认'}`);
            const element = document.getElementById('c' + i);
            if (element) {
                element.innerHTML =
                    `<a href="#" role="button" class="contrast" id="c_b${i}" style="${opacity}">${content}</a>`;
            }
        }

        // 清除多余的显示位置
        for (let i = displayCount; i < 12; i++) {
            const element = document.getElementById('c' + i);
            if (element) {
                element.innerHTML = "";
            }
        }

        // 设置样式
        const middleElement = document.getElementById('c_b' + middleIndex);
        if (middleElement) {
            if (inClassTime) {
                // 在上课时间时，中间位置高亮显示
                middleElement.style.backgroundColor = '#93cee97f';
                middleElement.style.fontWeight = '600';
                middleElement.style.opacity = '1';
                // console.log('[nowClass] 设置中间位置为上课状态样式');
            } else {
                // 不在上课时间时，中间位置显示休息样式
                middleElement.style.backgroundColor = '#93cee97f';
                middleElement.style.fontWeight = '600';
                middleElement.style.opacity = '1';
                // console.log('[nowClass] 设置中间位置为休息状态样式');
            }
        }
        
        // 设置中间位置之前的样式
        for (let i = 0; i < middleIndex; i++) {
            const el = document.getElementById('c_b' + i);
            if (el) {
                el.style.backgroundColor = '#3daee940';
                el.style.fontWeight = '400';
            }
        }
        
        // 设置中间位置之后的样式
        for (let i = middleIndex + 1; i < displayCount; i++) {
            const el = document.getElementById('c_b' + i);
            if (el) {
                el.style.backgroundColor = '';
                el.style.fontWeight = '400';
            }
        }
        // console.log('[nowClass] ===== 滚动模式渲染完成 =====');
    } else if (displayMode === 'day') {
        // 一天进度模式：只显示当天全部课程，前/当前/后课程有进度感
        // console.log('[nowClass] ===== 开始日间模式处理 =====');
        const todayClasses = today_vec.slice(0, -1);
        // console.log('[nowClass] 今日课程列表:', todayClasses);
        
        // 获取当前星期几
        const weekDays = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];
        const currentDay = weekDays[new Date().getDay()];
        
        let highlightIdx = -1;
        if (inClassTime) {
            highlightIdx = it;
            // console.log(`[nowClass] 在上课时间，高亮课程索引: ${highlightIdx}`);
        } else {
            // 不在上课时间，定位最近上一节/下一节
            const { prevIdx, nextIdx } = findNearestClasses(now, schedule);
            highlightIdx = nextIdx; // 可选：也可不高亮任何课程
            // console.log(`[nowClass] 不在上课时间，高亮课程索引: ${highlightIdx} (下一节: ${nextIdx}, 上一节: ${prevIdx})`);
        }

        // console.log('[nowClass] 开始渲染日间模式课程表');
        
        // 第一个位置显示星期几
        const c0Element = document.getElementById('c0');
        if (c0Element) {
            c0Element.innerHTML =
                `<a href="#" role="button" class="contrast" id="c_b0" style="background-color:#93cee97f; font-weight:600; opacity:1;">${currentDay}</a>`;
        }
        
        // 计算实际需要的显示位置数量（星期几 + 课程数量）
        const displayCount = todayClasses.length + 1;
        
        // 渲染课程，从第二个位置开始
        for (let i = 0; i < todayClasses.length; i++) {
            let content = todayClasses[i] || "";
            let elStyle = '';
            const displayIndex = i + 1; // 实际显示位置，从1开始
            
            if (highlightIdx === -1) {
                // 没有高亮，全部淡色
                elStyle = 'background-color:; font-weight:400; opacity:0.5;';
                // console.log(`[nowClass] 课程[${displayIndex}]: ${content} - 无高亮状态`);
            } else if (i < highlightIdx) {
                // 已上过
                elStyle = 'background-color:#3daee940; font-weight:400; opacity:0.8;';
                // console.log(`[nowClass] 课程[${displayIndex}]: ${content} - 已上过`);
            } else if (i == highlightIdx && !inClassTime) { // 新增无课条件
                // 无课时
                elStyle = 'background-color:#3daee940; font-weight:400; opacity:0.8;';
                // console.log(`[nowClass] 课程[${displayIndex}]: ${content} - 无课时间`);
            } else if (i === highlightIdx) {
                // 当前/即将上课
                elStyle = 'background-color:#93cee97f; font-weight:600; opacity:1;';
                // console.log(`[nowClass] 课程[${displayIndex}]: ${content} - 当前/即将上课`);
            } else {
                // 未上课
                elStyle = 'background-color:; font-weight:400; opacity:0.5;';
                // console.log(`[nowClass] 课程[${displayIndex}]: ${content} - 未上课`);
            }
            
            const element = document.getElementById('c' + displayIndex);
            if (element) {
                element.innerHTML =
                    `<a href="#" role="button" class="contrast" id="c_b${displayIndex}" style="${elStyle}">${content}</a>`;
            }
        }
        
        // 清除多余的显示位置
        for (let i = displayCount; i < 12; i++) {
            document.getElementById('c' + i).innerHTML = "";
        }
        
        // console.log('[nowClass] ===== 日间模式渲染完成 =====');
    }
    // console.log('[nowClass] ===== 课程显示刷新完成 =====\n');
}

// 首次加载
nowClass();
// 每秒刷新
refreshIntervalId = setInterval(nowClass, 1000);

// 调试函数：用于测试课程的显示
function debugClassDisplay() {
    console.log('===== 课程显示调试工具 =====');
    console.log('可用命令：');
    console.log('1. testScrollMode(hour, minute) - 测试滚动模式在指定时间的显示');
    console.log('2. testDayMode(hour, minute) - 测试日间模式在指定时间的显示');
    console.log('3. testCurrentTime() - 测试当前时间的课程显示');
    console.log('4. toggleDisplayMode() - 切换显示模式（滚动/日间）');
    console.log('5. showWeekSchedule(dayIndex) - 显示指定星期的课程表');
    console.log('6. simulateTimeProgress() - 模拟一天的时间进度');
    console.log('7. enterDebugMode() - 进入调试模式（暂停自动刷新）');
    console.log('8. exitDebugMode() - 退出调试模式（恢复自动刷新）');
    console.log('使用示例：testScrollMode(8, 30) 测试早上8:30的滚动模式显示');
    console.log('============================');
}

// 进入调试模式（暂停自动刷新）
function enterDebugMode() {
    if (isDebugMode) {
        console.log('已经在调试模式中');
        return;
    }
    
    // 清除定时器
    if (refreshIntervalId) {
        clearInterval(refreshIntervalId);
        refreshIntervalId = null;
    }
    
    isDebugMode = true;
    console.log('已进入调试模式，自动刷新已暂停');
    console.log('使用 exitDebugMode() 退出调试模式');
}

// 退出调试模式（恢复自动刷新）
function exitDebugMode() {
    if (!isDebugMode) {
        console.log('不在调试模式中');
        return;
    }
    
    // 恢复定时器
    refreshIntervalId = setInterval(nowClass, 1000);
    
    isDebugMode = false;
    console.log('已退出调试模式，自动刷新已恢复');
    
    // 刷新一次显示
    nowClass();
}

// 测试滚动模式在指定时间的显示
function testScrollMode(hour, minute) {
    if (hour === undefined || minute === undefined) {
        console.error('请提供小时和分钟参数，例如：testScrollMode(8, 30)');
        return;
    }
    
    // 自动进入调试模式
    enterDebugMode();
    
    // 保存原始的Date对象
    const originalDate = Date;
    
    // 创建模拟的Date对象
    const MockDate = function() {
        const date = new originalDate();
        date.setHours(hour, minute, 0, 0);
        return date;
    };
    
    // 复制原始Date的所有静态方法
    MockDate.prototype = originalDate.prototype;
    for (const key in originalDate) {
        MockDate[key] = originalDate[key];
    }
    
    // 临时替换Date对象
    Date = MockDate;
    
    // 临时设置为滚动模式
    const originalMode = CONFIG.lessons.displayMode;
    CONFIG.lessons.displayMode = 'scroll';
    
    // 刷新显示
    nowClass();
    
    // 恢复原始设置
    CONFIG.lessons.displayMode = originalMode;
    Date = originalDate;
    
    console.log(`已测试滚动模式在 ${hour}:${minute.toString().padStart(2, '0')} 的显示效果`);
    console.log('提示：使用 exitDebugMode() 恢复自动刷新');
}

// 测试日间模式在指定时间的显示
function testDayMode(hour, minute) {
    if (hour === undefined || minute === undefined) {
        console.error('请提供小时和分钟参数，例如：testDayMode(14, 30)');
        return;
    }
    
    // 自动进入调试模式
    enterDebugMode();
    
    // 保存原始的Date对象
    const originalDate = Date;
    
    // 创建模拟的Date对象
    const MockDate = function() {
        const date = new originalDate();
        date.setHours(hour, minute, 0, 0);
        return date;
    };
    
    // 复制原始Date的所有静态方法
    MockDate.prototype = originalDate.prototype;
    for (const key in originalDate) {
        MockDate[key] = originalDate[key];
    }
    
    // 临时替换Date对象
    Date = MockDate;
    
    // 临时设置为日间模式
    const originalMode = CONFIG.lessons.displayMode;
    CONFIG.lessons.displayMode = 'day';
    
    // 刷新显示
    nowClass();
    
    // 恢复原始设置
    CONFIG.lessons.displayMode = originalMode;
    Date = originalDate;
    
    console.log(`已测试日间模式在 ${hour}:${minute.toString().padStart(2, '0')} 的显示效果`);
    console.log('提示：使用 exitDebugMode() 恢复自动刷新');
}

// 测试当前时间的课程显示
function testCurrentTime() {
    // 自动进入调试模式
    enterDebugMode();
    
    const now = new Date();
    const hour = now.getHours();
    const minute = now.getMinutes();
    const mode = CONFIG.lessons.displayMode;
    
    console.log(`当前时间: ${hour}:${minute.toString().padStart(2, '0')}`);
    console.log(`当前模式: ${mode}`);
    
    // 刷新显示
    nowClass();
    
    console.log('已刷新当前时间的课程显示');
    console.log('提示：使用 exitDebugMode() 恢复自动刷新');
}

// 切换显示模式
function toggleDisplayMode() {
    // 自动进入调试模式
    enterDebugMode();
    
    const currentMode = CONFIG.lessons.displayMode;
    const newMode = currentMode === 'scroll' ? 'day' : 'scroll';
    
    CONFIG.lessons.displayMode = newMode;
    nowClass();
    
    console.log(`已切换显示模式: ${currentMode} -> ${newMode}`);
    console.log('提示：使用 exitDebugMode() 恢复自动刷新');
}

// 显示指定星期的课程表
function showWeekSchedule(dayIndex) {
    if (dayIndex === undefined || dayIndex < 0 || dayIndex > 6) {
        console.error('请提供有效的星期索引(0-6)，0表示周一，6表示周日');
        return;
    }
    
    const weekDays = ['周一', '周二', '周三', '周四', '周五', '周六', '周日'];
    const schedule = CONFIG.lessons.schedule[dayIndex];
    
    console.log(`===== ${weekDays[dayIndex]}课程表 =====`);
    schedule.classes.forEach((className, index) => {
        const timeInfo = CONFIG.lessons.times.schedule[index];
        console.log(`第${index + 1}节 (${timeInfo.begin}-${timeInfo.end}): ${className}`);
    });
    console.log('========================');
}

// 模拟一天的时间进度
function simulateTimeProgress() {
    // 自动进入调试模式
    enterDebugMode();
    
    const schedule = CONFIG.lessons.times.schedule;
    const mode = CONFIG.lessons.displayMode;
    
    console.log(`===== 开始模拟一天的时间进度 (当前模式: ${mode}) =====`);
    console.log('提示：使用 exitDebugMode() 停止模拟并恢复自动刷新');
    
    // 获取第一节课的开始时间和最后一节课的结束时间
    const firstClassBegin = schedule[0].begin.split(':');
    const lastClassEnd = schedule[schedule.length - 1].end.split(':');
    
    let currentHour = parseInt(firstClassBegin[0]);
    let currentMinute = parseInt(firstClassBegin[1]);
    const endHour = parseInt(lastClassEnd[0]);
    const endMinute = parseInt(lastClassEnd[1]);
    
    // 创建一个递归函数来模拟时间进度
    function simulateNextStep() {
        // 检查是否已经退出调试模式
        if (!isDebugMode) {
            console.log('===== 时间进度模拟已停止 =====');
            return;
        }
        
        if (currentHour > endHour || (currentHour === endHour && currentMinute >= endMinute)) {
            console.log('===== 时间进度模拟完成 =====');
            return;
        }
        
        // 保存原始的Date对象
        const originalDate = Date;
        
        // 创建模拟的Date对象
        const MockDate = function() {
            const date = new originalDate();
            date.setHours(currentHour, currentMinute, 0, 0);
            return date;
        };
        
        // 复制原始Date的所有静态方法
        MockDate.prototype = originalDate.prototype;
        for (const key in originalDate) {
            MockDate[key] = originalDate[key];
        }
        
        // 临时替换Date对象
        Date = MockDate;
        
        // 刷新显示
        nowClass();
        
        // 恢复Date对象
        Date = originalDate;
        
        console.log(`模拟时间: ${currentHour}:${currentMinute.toString().padStart(2, '0')}`);
        
        // 增加时间（每次增加30分钟）
        currentMinute += 30;
        if (currentMinute >= 60) {
            currentMinute -= 60;
            currentHour++;
        }
        
        // 2秒后继续下一步
        setTimeout(simulateNextStep, 2000);
    }
    
    // 开始模拟
    simulateNextStep();
}

// 将调试函数暴露到全局作用域，方便在控制台调用
window.debugClassDisplay = debugClassDisplay;
window.enterDebugMode = enterDebugMode;
window.exitDebugMode = exitDebugMode;
window.testScrollMode = testScrollMode;
window.testDayMode = testDayMode;
window.testCurrentTime = testCurrentTime;
window.toggleDisplayMode = toggleDisplayMode;
window.showWeekSchedule = showWeekSchedule;
window.simulateTimeProgress = simulateTimeProgress;

// 初始化时显示调试帮助
console.log('%c课程显示调试工具已加载！', 'color: #4CAF50; font-weight: bold');
console.log('%c在控制台输入 debugClassDisplay() 查看可用命令', 'color: #2196F3;');
console.log('%c注意：使用测试函数会自动进入调试模式，使用 exitDebugMode() 恢复自动刷新', 'color: #FF9800;');