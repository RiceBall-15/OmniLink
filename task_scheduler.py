#!/usr/bin/env python3
"""
OmniLink 任务调度器
确保开发任务从0点持续运行到8点
"""

import os
import sys
import time
import subprocess
from datetime import datetime, timedelta
from pathlib import Path

class OmniLinkScheduler:
    def __init__(self):
        self.project_root = Path("/root/omnilink")
        self.dev_agent = self.project_root / "dev_agent.py"
        self.task_queue = self.project_root / "TASK_QUEUE.md"
        self.output_dir = Path.home() / ".hermes/cron/output/db47923f7f84"
        self.session_log = self.output_dir / "session.log"
        
        # 时间窗口
        self.start_hour = 0
        self.end_hour = 8
        self.end_minute = 45
        
    def is_in_development_window(self):
        """检查是否在开发窗口内"""
        now = datetime.now()
        current_hour = now.hour
        current_minute = now.minute
        
        # 检查是否在0点到8:45之间
        if current_hour < self.start_hour:
            return False
        if current_hour >= self.end_hour and current_minute >= self.end_minute:
            return False
        
        return True
    
    def get_remaining_time(self):
        """获取剩余开发时间（分钟）"""
        now = datetime.now()
        end_time = now.replace(hour=self.end_hour, minute=self.end_minute, second=0, microsecond=0)
        
        if now >= end_time:
            return 0
        
        remaining = end_time - now
        return int(remaining.total_seconds() / 60)
    
    def get_next_task(self):
        """获取下一个待开发任务"""
        if not self.task_queue.exists():
            return None
        
        with open(self.task_queue, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        for i, line in enumerate(lines):
            # 查找包含 ⏳ 的行，但跳过以 # 开头的行（标题行）
            if '⏳' in line and not line.strip().startswith('#'):
                return {
                    'line_number': i,
                    'content': line.strip(),
                    'name': self.extract_task_name(line)
                }
            
            # 如果是任务标题行（以 #### 开头），检查下一行是否有子任务
            if line.strip().startswith('####') and '⏳' in line:
                # 提取任务名称（去掉 #### 和 ⏳）
                task_name = line.replace('####', '').replace('⏳', '').strip()
                # 去掉数字前缀（如 "3. "）
                if task_name and task_name[0].isdigit():
                    task_name = task_name.split('.', 1)[1].strip() if '.' in task_name else task_name
                
                return {
                    'line_number': i,
                    'content': line.strip(),
                    'name': task_name
                }
        
        return None
    
    def extract_task_name(self, line):
        """从任务行中提取任务名称"""
        if '- [ ]' in line:
            return line.split('- [ ]')[1].strip()
        elif '- ' in line:
            return line.split('- ')[1].strip()
        return line.strip()
    
    def mark_task_in_progress(self, task):
        """标记任务为进行中"""
        if not task or not self.task_queue.exists():
            return False
        
        with open(self.task_queue, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        if task['line_number'] < len(lines):
            lines[task['line_number']] = lines[task['line_number']].replace('⏳', '🔄')
            
            with open(self.task_queue, 'w', encoding='utf-8') as f:
                f.writelines(lines)
            return True
        return False
    
    def mark_task_completed(self, task, commit_hash=None):
        """标记任务为已完成"""
        if not task or not self.task_queue.exists():
            return False
        
        with open(self.task_queue, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        if task['line_number'] < len(lines):
            lines[task['line_number']] = lines[task['line_number']].replace('🔄', '✅')
            if commit_hash:
                lines[task['line_number']] = lines[task['line_number']].rstrip() + f' (commit: {commit_hash[:8]})'
            
            with open(self.task_queue, 'w', encoding='utf-8') as f:
                f.writelines(lines)
            return True
        return False
    
    def mark_task_blocked(self, task, reason):
        """标记任务为受阻"""
        if not task or not self.task_queue.exists():
            return False
        
        with open(self.task_queue, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        if task['line_number'] < len(lines):
            lines[task['line_number']] = lines[task['line_number']].replace('🔄', '⚠️')
            lines[task['line_number']] = lines[task['line_number']].rstrip() + f' - 受阻: {reason}'
            
            with open(self.task_queue, 'w', encoding='utf-8') as f:
                f.writelines(lines)
            return True
        return False
    
    def run_dev_agent(self):
        """运行开发代理"""
        if not self.dev_agent.exists():
            return False, "开发代理脚本不存在"
        
        try:
            result = subprocess.run(
                ['python3', str(self.dev_agent)],
                cwd=self.project_root,
                capture_output=True,
                text=True,
                timeout=1800  # 30分钟超时
            )
            
            return result.returncode == 0, result.stdout + result.stderr
            
        except subprocess.TimeoutExpired:
            return False, "开发代理运行超时"
        except Exception as e:
            return False, f"运行开发代理失败: {str(e)}"
    
    def generate_progress_report(self, cycle, task, success, message):
        """生成进度报告"""
        timestamp = datetime.now().strftime("%Y-%m-%d_%H-%M-%S")
        report_file = self.output_dir / f"{timestamp}.md"
        
        remaining_minutes = self.get_remaining_time()
        remaining_hours = remaining_minutes / 60
        
        with open(report_file, 'w', encoding='utf-8') as f:
            f.write(f"# OmniLink 开发进度报告\n\n")
            f.write(f"**报告时间**: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n")
            f.write(f"**开发周期**: #{cycle}\n")
            f.write(f"**当前任务**: {task['name'] if task else '无'}\n")
            f.write(f"**任务状态**: {'✅ 成功' if success else '❌ 失败'}\n")
            f.write(f"**详细信息**: {message}\n\n")
            
            f.write(f"## 📊 本周期统计\n\n")
            f.write(f"- **状态**: {'✅ 成功' if success else '❌ 失败'}\n")
            f.write(f"- **提交**: 待Git提交\n")
            f.write(f"- **下一步**: {'继续下一个任务' if success else '检查问题并修复'}\n\n")
            
            f.write(f"## ⏰ 时间管理\n\n")
            f.write(f"- 当前时间: {datetime.now().strftime('%H:%M:%S')}\n")
            f.write(f"- 剩余时间: {remaining_hours:.1f}小时 ({remaining_minutes}分钟)\n\n")
            
            f.write(f"---\n")
            f.write(f"*报告生成时间: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}*\n")
        
        return report_file
    
    def generate_final_report(self, completed_tasks, blocked_tasks):
        """生成最终报告"""
        timestamp = datetime.now().strftime("%Y-%m-%d_%H-%M-%S")
        report_file = self.output_dir / f"final_{timestamp}.md"
        
        with open(report_file, 'w', encoding='utf-8') as f:
            f.write(f"# OmniLink 开发会话最终报告\n\n")
            f.write(f"**会话时间**: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n")
            f.write(f"**开发周期**: {len(completed_tasks) + len(blocked_tasks)} 个周期\n")
            f.write(f"**总耗时**: 8小时\n\n")
            
            f.write(f"## 📊 完成情况\n\n")
            f.write(f"| 状态 | 数量 |\n")
            f.write(f"|------|------|\n")
            f.write(f"| ✅ 完成 | {len(completed_tasks)} |\n")
            f.write(f"| ⚠️ 受阻 | {len(blocked_tasks)} |\n")
            f.write(f"| ⏳ 待开发 | 待统计 |\n\n")
            
            if completed_tasks:
                f.write(f"## ✅ 完成的任务\n\n")
                for task in completed_tasks:
                    f.write(f"- {task}\n")
                f.write("\n")
            
            if blocked_tasks:
                f.write(f"## ⚠️ 受阻的任务\n\n")
                for task in blocked_tasks:
                    f.write(f"- {task}\n")
                f.write("\n")
            
            f.write(f"## 📝 开发日志\n\n")
            f.write(f"详细日志请查看: {self.session_log}\n\n")
            
            f.write(f"## 🎯 下一步\n\n")
            f.write(f"1. 检查受阻任务的原因\n")
            f.write(f"2. 继续处理待开发任务\n")
            f.write(f"3. 优化开发流程\n\n")
            
            f.write(f"---\n")
            f.write(f"*最终报告生成时间: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}*\n")
        
        return report_file
    
    def run(self):
        """运行任务调度器"""
        print("🎯 启动 OmniLink 任务调度器")
        
        # 创建输出目录
        self.output_dir.mkdir(parents=True, exist_ok=True)
        
        # 记录会话开始
        with open(self.session_log, 'a', encoding='utf-8') as f:
            f.write(f"\n=== OmniLink 开发会话开始 ===\n")
            f.write(f"时间: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n")
            f.write(f"开发窗口: 00:00 - 08:45\n\n")
        
        # 检查是否在开发窗口内
        if not self.is_in_development_window():
            print("❌ 当前时间不在开发窗口内")
            print(f"开发窗口: {self.start_hour}:00 - {self.end_hour}:{self.end_minute}")
            return
        
        remaining_minutes = self.get_remaining_time()
        print(f"⏰ 剩余开发时间: {remaining_minutes}分钟")
        
        cycle = 1
        completed_tasks = []
        blocked_tasks = []
        
        # 主循环：持续执行任务直到时间结束
        while self.is_in_development_window():
            print(f"\n{'='*50}")
            print(f"🔄 开发周期 #{cycle}")
            print(f"⏰ 当前时间: {datetime.now().strftime('%H:%M:%S')}")
            print(f"⏰ 剩余时间: {self.get_remaining_time()}分钟")
            print(f"{'='*50}")
            
            # 检查剩余时间，如果少于15分钟则准备结束
            if self.get_remaining_time() < 15:
                print("⏰ 剩余时间不足15分钟，准备生成最终报告")
                break
            
            # 获取下一个任务
            task = self.get_next_task()
            if not task:
                print("✅ 所有任务已完成")
                break
            
            print(f"📋 当前任务: {task['name']}")
            
            # 标记任务为进行中
            self.mark_task_in_progress(task)
            
            # 运行开发代理
            print("🔧 运行开发代理...")
            success, message = self.run_dev_agent()
            
            # 生成进度报告
            report_file = self.generate_progress_report(cycle, task, success, message)
            
            if success:
                # 标记任务为完成（这里应该从消息中提取commit hash）
                self.mark_task_completed(task)
                completed_tasks.append(task['name'])
                print(f"✅ 任务完成")
            else:
                # 标记任务为受阻
                self.mark_task_blocked(task, "开发失败")
                blocked_tasks.append(task['name'])
                print(f"⚠️ 任务受阻")
            
            print(f"📊 进度报告已生成: {report_file}")
            
            # 等待一段时间后继续下一个任务
            print("⏳ 等待10秒后继续...")
            time.sleep(10)
            
            cycle += 1
        
        # 生成最终报告
        final_report = self.generate_final_report(completed_tasks, blocked_tasks)
        
        # 记录会话结束
        with open(self.session_log, 'a', encoding='utf-8') as f:
            f.write(f"\n=== OmniLink 开发会话结束 ===\n")
            f.write(f"时间: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n")
            f.write(f"完成任务: {len(completed_tasks)}\n")
            f.write(f"受阻任务: {len(blocked_tasks)}\n")
            f.write(f"总周期: {cycle}\n")
        
        print(f"\n🎯 开发会话结束")
        print(f"✅ 完成任务: {len(completed_tasks)}")
        print(f"⚠️ 受阻任务: {len(blocked_tasks)}")
        print(f"📊 最终报告: {final_report}")


def main():
    """主函数"""
    scheduler = OmniLinkScheduler()
    scheduler.run()


if __name__ == "__main__":
    main()