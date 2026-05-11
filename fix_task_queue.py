#!/usr/bin/env python3
"""
修复任务队列
将被错误标记为已完成的任务改回待开发状态
"""

import re
from pathlib import Path

def fix_task_queue():
    """修复任务队列"""
    task_queue_file = Path("/root/omnilink/TASK_QUEUE.md")
    
    if not task_queue_file.exists():
        print("❌ 任务队列文件不存在")
        return
    
    with open(task_queue_file, 'r', encoding='utf-8') as f:
        lines = f.readlines()
    
    fixed_lines = []
    changes_made = 0
    
    for i, line in enumerate(lines):
        # 修复被错误标记为已完成的任务
        if '####' in line and '✅' in line:
            # 检查是否是主要功能任务（不是技术债务）
            if '技术债务' not in line and '测试和文档' not in line:
                # 检查下一行是否有子任务
                if i + 1 < len(lines) and '- [ ]' in lines[i + 1]:
                    # 这是一个有子任务的任务，应该标记为待开发
                    line = line.replace('✅', '⏳')
                    changes_made += 1
                    print(f"✅ 修复任务: {line.strip()}")
        
        fixed_lines.append(line)
    
    # 写入修复后的内容
    with open(task_queue_file, 'w', encoding='utf-8') as f:
        f.writelines(fixed_lines)
    
    print(f"\n🎉 修复完成！共修复 {changes_made} 个任务")

if __name__ == "__main__":
    fix_task_queue()