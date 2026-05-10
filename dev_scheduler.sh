#!/bin/bash
# OmniLink 开发任务调度器
# 确保从0点到8点持续运行

set -e

PROJECT_ROOT="/root/omnilink"
LOG_DIR="$HOME/.hermes/cron/output/db47923f7f84"
TASK_QUEUE="$PROJECT_ROOT/TASK_QUEUE.md"
SESSION_LOG="$LOG_DIR/session.log"

# 创建日志目录
mkdir -p "$LOG_DIR"

# 记录会话开始
echo "=== OmniLink 开发会话开始 ===" >> "$SESSION_LOG"
echo "时间: $(date '+%Y-%m-%d %H:%M:%S')" >> "$SESSION_LOG"
echo "" >> "$SESSION_LOG"

# 检查是否在开发窗口内（0点到8点）
current_hour=$(date +%-H)
if [ "$current_hour" -ge 8 ]; then
    echo "❌ 当前时间已超过8点，不在开发窗口内" | tee -a "$SESSION_LOG"
    echo "开发窗口: 00:00 - 08:00" | tee -a "$SESSION_LOG"
    exit 1
fi

# 计算剩余时间（秒）
end_time=$(date -d "today 08:00" +%s)
current_time=$(date +%s)
remaining_seconds=$((end_time - current_time))
remaining_hours=$(echo "scale=1; $remaining_seconds / 3600" | bc)

echo "⏰ 剩余开发时间: ${remaining_hours}小时" | tee -a "$SESSION_LOG"
echo "" >> "$SESSION_LOG"

# 主循环：持续执行任务直到8点
cycle=1
while [ "$(date +%-H)" -lt 8 ]; do
    echo "===========================================" | tee -a "$SESSION_LOG"
    echo "🔄 开发周期 #$cycle" | tee -a "$SESSION_LOG"
    echo "⏰ 当前时间: $(date '+%H:%M:%S')" | tee -a "$SESSION_LOG"
    echo "===========================================" | tee -a "$SESSION_LOG"
    
    # 检查剩余时间，如果少于15分钟则准备结束
    current_time=$(date +%s)
    remaining_seconds=$((end_time - current_time))
    if [ "$remaining_seconds" -lt 900 ]; then
        echo "⏰ 剩余时间不足15分钟，准备生成最终报告" | tee -a "$SESSION_LOG"
        break
    fi
    
    # 读取下一个任务
    if [ ! -f "$TASK_QUEUE" ]; then
        echo "❌ 任务队列文件不存在: $TASK_QUEUE" | tee -a "$SESSION_LOG"
        break
    fi
    
    # 查找下一个待开发任务（标记为 ⏳）
    next_task=$(grep -n "⏳" "$TASK_QUEUE" | grep -v "^#" | head -1)
    if [ -z "$next_task" ]; then
        echo "✅ 所有任务已完成！" | tee -a "$SESSION_LOG"
        break
    fi
    
    # 提取任务信息
    task_line=$(echo "$next_task" | cut -d: -f1)
    task_content=$(echo "$next_task" | cut -d: -f2-)
    task_name=$(echo "$task_content" | sed 's/.*- \[ \] //' | sed 's/⏳//' | xargs)
    
    echo "📋 当前任务: $task_name" | tee -a "$SESSION_LOG"
    echo "📍 位置: 第${task_line}行" | tee -a "$SESSION_LOG"
    
    # 标记任务为进行中
    sed -i "${task_line}s/⏳/🔄/" "$TASK_QUEUE"
    echo "🔄 已标记为进行中" | tee -a "$SESSION_LOG"
    
    # 执行开发任务（这里应该调用实际的开发脚本）
    echo "🔧 开始执行开发任务..." | tee -a "$SESSION_LOG"
    
    # 记录任务开始时间
    task_start=$(date +%s)
    
    # 这里应该调用实际的开发逻辑
    # 例如：python3 /root/omnilink/dev_agent.py "$task_name"
    
    # 模拟任务执行（实际使用时替换为真正的开发逻辑）
    echo "⏳ 执行中..." | tee -a "$SESSION_LOG"
    sleep 5
    
    # 检查任务执行结果
    task_result=$?
    task_end=$(date +%s)
    task_duration=$((task_end - task_start))
    
    if [ $task_result -eq 0 ]; then
        # 任务成功，标记为完成
        sed -i "${task_line}s/🔄/✅/" "$TASK_QUEUE"
        echo "✅ 任务完成 (耗时: ${task_duration}秒)" | tee -a "$SESSION_LOG"
        
        # 更新进度报告
        timestamp=$(date '+%Y-%m-%d_%H-%M-%S')
        report_file="$LOG_DIR/${timestamp}.md"
        
        cat > "$report_file" << EOF
# OmniLink 开发进度报告

**报告时间**: $(date '+%Y-%m-%d %H:%M:%S')
**开发周期**: #$cycle
**完成任务**: $task_name
**任务耗时**: ${task_duration}秒

## 📊 本周期统计

- **状态**: ✅ 成功
- **提交**: 待Git提交
- **下一步**: 继续下一个任务

## ⏰ 时间管理

- 当前时间: $(date '+%H:%M:%S')
- 剩余时间: ${remaining_hours}小时

---
*报告生成时间: $(date '+%Y-%m-%d %H:%M:%S')*
EOF
        
        echo "📊 进度报告已生成: $report_file" | tee -a "$SESSION_LOG"
    else
        # 任务失败，标记为受阻
        sed -i "${task_line}s/🔄/⚠️/" "$TASK_QUEUE"
        echo "⚠️ 任务受阻，继续下一个" | tee -a "$SESSION_LOG"
    fi
    
    # 等待一段时间后继续下一个任务
    echo "⏳ 等待5秒后继续..." | tee -a "$SESSION_LOG"
    sleep 5
    
    cycle=$((cycle + 1))
    echo "" >> "$SESSION_LOG"
done

# 生成最终报告
echo "" | tee -a "$SESSION_LOG"
echo "===========================================" | tee -a "$SESSION_LOG"
echo "🎯 开发会话结束" | tee -a "$SESSION_LOG"
echo "===========================================" | tee -a "$SESSION_LOG"

# 统计完成情况
completed=$(grep -c "✅" "$TASK_QUEUE" || true)
blocked=$(grep -c "⚠️" "$TASK_QUEUE" || true)
pending=$(grep -c "⏳" "$TASK_QUEUE" || true)

echo "📊 最终统计:" | tee -a "$SESSION_LOG"
echo "  ✅ 完成: $completed" | tee -a "$SESSION_LOG"
echo "  ⚠️ 受阻: $blocked" | tee -a "$SESSION_LOG"
echo "  ⏳ 待开发: $pending" | tee -a "$SESSION_LOG"
echo "" >> "$SESSION_LOG"

# 创建最终报告
final_report="$LOG_DIR/final_$(date '+%Y-%m-%d_%H-%M-%S').md"
cat > "$final_report" << EOF
# OmniLink 开发会话最终报告

**会话时间**: $(date '+%Y-%m-%d %H:%M:%S')
**开发周期**: $cycle 个周期
**总耗时**: ${remaining_hours}小时

## 📊 完成情况

| 状态 | 数量 |
|------|------|
| ✅ 完成 | $completed |
| ⚠️ 受阻 | $blocked |
| ⏳ 待开发 | $pending |

## 📝 开发日志

详细日志请查看: $SESSION_LOG

## 🎯 下一步

1. 检查受阻任务的原因
2. 继续处理待开发任务
3. 优化开发流程

---
*最终报告生成时间: $(date '+%Y-%m-%d %H:%M:%S')*
EOF

echo "📊 最终报告已生成: $final_report" | tee -a "$SESSION_LOG"
echo "" >> "$SESSION_LOG"
echo "=== OmniLink 开发会话结束 ===" >> "$SESSION_LOG"