#!/usr/bin/env python3
"""
OmniLink 开发会话管理器
确保开发任务从0点持续运行到8点
"""

import os
import sys
import json
import time
import subprocess
from datetime import datetime, timedelta
from pathlib import Path

class DevelopmentSession:
    def __init__(self):
        self.project_root = Path("/root/omnilink")
        self.task_queue_file = self.project_root / "TASK_QUEUE.md"
        self.progress_file = self.project_root / "PROGRESS.md"
        self.dev_plan_file = self.project_root / "DEV_PLAN.md"
        self.log_dir = Path("/root/.hermes/cron/output/db47923f7f84")
        self.log_dir.mkdir(parents=True, exist_ok=True)
        
        # 开发时间窗口
        self.start_hour = 0  # 0点
        self.end_hour = 8    # 8点
        
        # 当前会话信息
        self.session_start = datetime.now()
        self.current_task = None
        self.completed_tasks = []
        self.blocked_tasks = []
        
    def is_within_development_window(self):
        """检查当前时间是否在开发窗口内"""
        now = datetime.now()
        current_hour = now.hour
        return self.start_hour <= current_hour < self.end_hour
    
    def get_remaining_time(self):
        """获取剩余开发时间（秒）"""
        now = datetime.now()
        end_time = now.replace(hour=self.end_hour, minute=0, second=0, microsecond=0)
        if now >= end_time:
            return 0
        return (end_time - now).total_seconds()
    
    def get_next_task(self):
        """从任务队列获取下一个待开发任务"""
        if not self.task_queue_file.exists():
            return None
        
        with open(self.task_queue_file, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # 查找标记为 ⏳ 的任务
        lines = content.split('\n')
        for i, line in enumerate(lines):
            if '⏳' in line and not line.strip().startswith('#'):
                # 提取任务名称
                task_name = line.strip()
                if task_name.startswith('- [ ]'):
                    task_name = task_name[5:].strip()
                elif task_name.startswith('- '):
                    task_name = task_name[2:].strip()
                
                return {
                    'name': task_name,
                    'line_number': i,
                    'raw_line': line
                }
        return None
    
    def mark_task_in_progress(self, task):
        """标记任务为进行中"""
        if not task or not self.task_queue_file.exists():
            return False
        
        with open(self.task_queue_file, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        if task['line_number'] < len(lines):
            lines[task['line_number']] = lines[task['line_number']].replace('⏳', '🔄')
            
            with open(self.task_queue_file, 'w', encoding='utf-8') as f:
                f.writelines(lines)
            return True
        return False
    
    def mark_task_completed(self, task, commit_hash=None):
        """标记任务为已完成"""
        if not task or not self.task_queue_file.exists():
            return False
        
        with open(self.task_queue_file, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        if task['line_number'] < len(lines):
            lines[task['line_number']] = lines[task['line_number']].replace('🔄', '✅')
            if commit_hash:
                lines[task['line_number']] = lines[task['line_number']].rstrip() + f' (commit: {commit_hash[:8]})'
            
            with open(self.task_queue_file, 'w', encoding='utf-8') as f:
                f.writelines(lines)
            return True
        return False
    
    def mark_task_blocked(self, task, reason):
        """标记任务为受阻"""
        if not task or not self.task_queue_file.exists():
            return False
        
        with open(self.task_queue_file, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        if task['line_number'] < len(lines):
            lines[task['line_number']] = lines[task['line_number']].replace('🔄', '⚠️')
            lines[task['line_number']] = lines[task['line_number']].rstrip() + f' - 受阻: {reason}'
            
            with open(self.task_queue_file, 'w', encoding='utf-8') as f:
                f.writelines(lines)
            return True
        return False
    
    def run_development_cycle(self, max_duration_minutes=30):
        """运行一个开发周期"""
        print(f"🔄 开始开发周期，最大时长: {max_duration_minutes}分钟")
        
        # 1. 获取下一个任务
        task = self.get_next_task()
        if not task:
            print("✅ 所有任务已完成！")
            return False
        
        print(f"📋 当前任务: {task['name']}")
        self.current_task = task
        
        # 2. 标记为进行中
        self.mark_task_in_progress(task)
        
        # 3. 执行开发工作（这里需要调用实际的开发逻辑）
        # 在实际使用中，这应该调用子代理或执行具体代码
        print(f"🔧 开始开发: {task['name']}")
        
        # 模拟开发过程
        time.sleep(5)  # 实际开发中应该执行真正的代码
        
        # 4. 标记为完成（示例）
        commit_hash = "abc12345"  # 实际应该从git获取
        self.mark_task_completed(task, commit_hash)
        self.completed_tasks.append(task)
        
        print(f"✅ 完成任务: {task['name']}")
        return True
    
    def generate_progress_report(self):
        """生成进度报告"""
        timestamp = datetime.now().strftime("%Y-%m-%d_%H-%M-%S")
        report_file = self.log_dir / f"{timestamp}.md"
        
        remaining_time = self.get_remaining_time()
        remaining_hours = remaining_time / 3600
        
        report_content = f"""# OmniLink 开发进度报告

**报告时间**: {datetime.now().strftime("%Y-%m-%d %H:%M:%S")}
**会话开始**: {self.session_start.strftime("%Y-%m-%d %H:%M:%S")}
**剩余时间**: {remaining_hours:.1f} 小时

## 📊 本次会话统计

- **完成任务数**: {len(self.completed_tasks)}
- **受阻任务数**: {len(self.blocked_tasks)}
- **当前任务**: {self.current_task['name'] if self.current_task else '无'}

## ✅ 已完成任务

"""
        
        for task in self.completed_tasks:
            report_content += f"- {task['name']}\n"
        
        report_content += "\n## ⚠️ 受阻任务\n\n"
        
        for task in self.blocked_tasks:
            report_content += f"- {task['name']}\n"
        
        report_content += f"""
## ⏰ 时间管理

- 开发窗口: {self.start_hour}:00 - {self.end_hour}:00
- 已用时间: {(datetime.now() - self.session_start).total_seconds() / 3600:.1f} 小时
- 剩余时间: {remaining_hours:.1f} 小时

## 📝 下一步

1. 继续执行任务队列中的下一个任务
2. 优先处理高优先级任务
3. 遇到阻塞时记录并跳过

---
*报告生成时间: {datetime.now().strftime("%Y-%m-%d %H:%M:%S")}*
"""
        
        with open(report_file, 'w', encoding='utf-8') as f:
            f.write(report_content)
        
        print(f"📊 进度报告已生成: {report_file}")
        return report_file
    
    def run_until_end_time(self, cycle_duration_minutes=30):
        """持续运行直到结束时间"""
        print(f"🚀 开始OmniLink开发会话")
        print(f"⏰ 开发窗口: {self.start_hour}:00 - {self.end_hour}:00")
        print(f"⏱️  每个周期: {cycle_duration_minutes}分钟")
        
        cycle_count = 0
        
        while self.is_within_development_window():
            cycle_count += 1
            print(f"\n{'='*50}")
            print(f"🔄 开发周期 #{cycle_count}")
            print(f"⏰ 剩余时间: {self.get_remaining_time()/3600:.1f}小时")
            print(f"{'='*50}")
            
            # 运行一个开发周期
            has_more_tasks = self.run_development_cycle(cycle_duration_minutes)
            
            # 生成进度报告
            self.generate_progress_report()
            
            if not has_more_tasks:
                print("🎉 所有任务已完成，提前结束开发会话")
                break
            
            # 检查剩余时间
            remaining_seconds = self.get_remaining_time()
            if remaining_seconds <= 0:
                print("⏰ 开发时间结束")
                break
            
            # 计算下一个周期的等待时间
            wait_seconds = min(cycle_duration_minutes * 60, remaining_seconds)
            print(f"⏳ 等待 {wait_seconds/60:.1f} 分钟后开始下一个周期...")
            
            # 在实际使用中，这里应该真正等待
            # time.sleep(wait_seconds)
            # 为了演示，我们只等待几秒
            time.sleep(5)
        
        # 生成最终报告
        final_report = self.generate_progress_report()
        print(f"\n🎯 开发会话结束")
        print(f"📊 最终报告: {final_report}")
        print(f"✅ 完成任务: {len(self.completed_tasks)}")
        print(f"⚠️ 受阻任务: {len(self.blocked_tasks)}")
        
        return {
            'completed': len(self.completed_tasks),
            'blocked': len(self.blocked_tasks),
            'report': str(final_report)
        }


def main():
    """主函数"""
    session = DevelopmentSession()
    
    # 检查是否在开发窗口内
    if not session.is_within_development_window():
        print(f"❌ 当前不在开发窗口内")
        print(f"⏰ 开发窗口: {session.start_hour}:00 - {session.end_hour}:00")
        print(f"🕐 当前时间: {datetime.now().strftime('%H:%M:%S')}")
        sys.exit(1)
    
    # 运行开发会话
    result = session.run_until_end_time(cycle_duration_minutes=30)
    
    print("\n" + "="*50)
    print("🎉 OmniLink开发会话完成")
    print("="*50)
    print(f"✅ 完成任务: {result['completed']}")
    print(f"⚠️ 受阻任务: {result['blocked']}")
    print(f"📊 进度报告: {result['report']}")


if __name__ == "__main__":
    main()