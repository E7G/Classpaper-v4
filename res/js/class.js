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

    const dayCount = 7;           // 一周有7天
    const lessonsPerDay = 12;     // 每天有12节课
    const total = (dayCount+1) * lessonsPerDay; // 一周总共的课程数

    // 计算今天在课程数组中的起始下标
    // 例如：周一为12，周二为24，依此类推
    let offset =  week  * lessonsPerDay;
    // console.log(`[getDayVectors] week: ${week}, offset: ${offset}`);

    // 取出今天的课程数组
    // 这里slice的长度为13，是因为每天12节课+1个空元素（可能用于占位或防止越界）
    let today_vec = source_vec.slice(offset, offset + lessonsPerDay).map(item => item.replace(/\n/g, ""));
    today_vec.push("");
    // console.log(`[getDayVectors] today_vec:`, today_vec);

    // 计算前一天的起始下标
    let prev_offset = offset - lessonsPerDay;
    // 如果前一天小于0，说明已经到周一的前一天了，需要循环到周日
    if (prev_offset < 1*lessonsPerDay) prev_offset = total-lessonsPerDay;
    // console.log(`[getDayVectors] prev_offset: ${prev_offset}`);

    // 计算后一天的起始下标
    let next_offset = offset + lessonsPerDay;
    // 如果后一天超出总课程数，说明已经到周日的后一天了，需要循环到周一
    if (next_offset >= total) next_offset = 1*lessonsPerDay;
    // console.log(`[getDayVectors] next_offset: ${next_offset}`);

    // 取出前一天的课程数组，并在末尾加一个空元素
    let prev_vec = source_vec.slice(prev_offset, prev_offset + lessonsPerDay).map(item => item.replace(/\n/g, ""));
    prev_vec.push("");
    // 取出后一天的课程数组，并在末尾加一个空元素
    let next_vec = source_vec.slice(next_offset, next_offset + lessonsPerDay).map(item => item.replace(/\n/g, ""));
    next_vec.push("");

    // console.log(`[getDayVectors] prev_vec:`, prev_vec);
    // console.log(`[getDayVectors] next_vec:`, next_vec);

    // 返回今天、前一天、后一天的课程数组
    return {
        today_vec, // 今天的课程
        prev_vec,  // 前一天的课程
        next_vec   // 后一天的课程
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
            //schedule和todayClasses的差距修正
            currentIndex++;
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
            // console.log('[nowClass] 无当前课程，开始填充休息状态和前后课程');
            // 中间位置显示休息
            arranged[6] = "休息";
            // console.log('[nowClass] 设置中间位置为"休息"');
            
            // 向前追溯填充前一天课程
            for (let i = 5; i >= 0; i--) {
                if (!arranged[i] && prev_vec) {
                    const prevIndex = prev_vec.length - (6 - i);
                    arranged[i] = prevIndex >= 0 ? prev_vec[prevIndex] : "...";
                    // console.log(`[nowClass] 填充前向课程[${i}]: ${arranged[i]} (索引: ${prevIndex})`);
                }
            }
            
            // 向后追溯填充后一天课程
            for (let i = 7; i < 12; i++) {
                if (!arranged[i] && next_vec) {
                    const nextIndex = i - 7;
                    arranged[i] = nextIndex < next_vec.length ? next_vec[nextIndex] : "...";
                    // console.log(`[nowClass] 填充后向课程[${i}]: ${arranged[i]} (索引: ${nextIndex})`);
                }
            }
            // console.log('[nowClass] 填充完成后的课程排列:', arranged);
        } else {
            // console.log('[nowClass] 当前在上课时间，不显示休息，直接使用实际课程');
        }

        // 渲染课程表
        // console.log('[nowClass] ===== 开始渲染课程表 =====');
        for (let i = 0; i < arranged.length; i++) {
            let content = arranged[i] || "";
            let opacity = (i === 6) ? "" : "opacity: 0.5;";
            // console.log(`[nowClass] 渲染课程[${i}]: ${content}, 样式: ${opacity || '默认'}`);
            document.getElementById('c' + i).innerHTML =
                `<a href="#" role="button" class="contrast" id="c_b${i}" style="${opacity}">${content}</a>`;
        }

        // 设置样式
        const c_b6 = document.getElementById('c_b6');
        if (inClassTime) {
            // 在上课时间时，中间位置高亮显示
            c_b6.style.backgroundColor = '#93cee97f';
            c_b6.style.fontWeight = '600';
            c_b6.style.opacity = '1';
            // console.log('[nowClass] 设置中间位置为上课状态样式');
        } else {
            // 不在上课时间时，中间位置显示休息样式
            c_b6.style.backgroundColor = '#93cee97f';
            c_b6.style.fontWeight = '600';
            c_b6.style.opacity = '1';
            // console.log('[nowClass] 设置中间位置为休息状态样式');
        }
        
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
        // console.log('[nowClass] ===== 滚动模式渲染完成 =====');
    } else if (displayMode === 'day') {
        // 一天进度模式：只显示当天全部课程，前/当前/后课程有进度感
        // console.log('[nowClass] ===== 开始日间模式处理 =====');
        const todayClasses = today_vec.slice(0, -1);
        // console.log('[nowClass] 今日课程列表:', todayClasses);
        
        let highlightIdx = -1;
        if (inClassTime) {
            highlightIdx = it;
            //schedule和todayClasses的差距修正
            highlightIdx++;
            // console.log(`[nowClass] 在上课时间，高亮课程索引: ${highlightIdx}`);
        } else {
            // 不在上课时间，定位最近上一节/下一节
            const { prevIdx, nextIdx } = findNearestClasses(now, schedule);
            highlightIdx = nextIdx; // 可选：也可不高亮任何课程
            // console.log(`[nowClass] 不在上课时间，高亮课程索引: ${highlightIdx} (下一节: ${nextIdx}, 上一节: ${prevIdx})`);
        }

        // console.log('[nowClass] 开始渲染日间模式课程表');
        for (let i = 0; i < todayClasses.length; i++) {
            let content = todayClasses[i] || "";
            let elStyle = '';
            
            if (highlightIdx === -1) {
                // 没有高亮，全部淡色
                elStyle = 'background-color:; font-weight:400; opacity:0.5;';
                // console.log(`[nowClass] 课程[${i}]: ${content} - 无高亮状态`);
            } else if (i < highlightIdx) {
                // 已上过
                elStyle = 'background-color:#3daee940; font-weight:400; opacity:0.8;';
                // console.log(`[nowClass] 课程[${i}]: ${content} - 已上过`);
            } else if (i == highlightIdx && !inClassTime) { // 新增无课条件
                // 无课时
                elStyle = 'background-color:#3daee940; font-weight:400; opacity:0.8;';
                // console.log(`[nowClass] 课程[${i}]: ${content} - 无课时间`);
            } else if (i === highlightIdx) {
                // 当前/即将上课
                elStyle = 'background-color:#93cee97f; font-weight:600; opacity:1;';
                // console.log(`[nowClass] 课程[${i}]: ${content} - 当前/即将上课`);
            } else {
                // 未上课
                elStyle = 'background-color:; font-weight:400; opacity:0.5;';
                // console.log(`[nowClass] 课程[${i}]: ${content} - 未上课`);
            }
            
            document.getElementById('c' + i).innerHTML =
                `<a href="#" role="button" class="contrast" id="c_b${i}" style="${elStyle}">${content}</a>`;
        }
        // console.log('[nowClass] ===== 日间模式渲染完成 =====');
    }
    // console.log('[nowClass] ===== 课程显示刷新完成 =====\n');
}

// 首次加载
nowClass();
// 每秒刷新
setInterval(nowClass, 1000);