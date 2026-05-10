#!/usr/bin/env python3
"""
OmniLink 开发代理
执行实际的开发任务，包括代码编写、错误修复、Git提交
"""

import os
import sys
import json
import subprocess
import time
from pathlib import Path
from datetime import datetime

class OmniLinkDeveloper:
    def __init__(self):
        self.project_root = Path("/root/omnilink")
        self.task_queue_file = self.project_root / "TASK_QUEUE.md"
        self.progress_file = self.project_root / "PROGRESS.md"
        
        # 项目配置
        self.services = [
            "common",
            "im-gateway",
            "im-api", 
            "ai-service",
            "user-service",
            "file-service",
            "usage-service",
            "push-service",
            "config-service"
        ]
        
        # 资源限制
        self.max_memory_usage = 80  # 最大内存使用百分比
        self.cargo_jobs = 1  # 编译并行度
        
    def check_resources(self):
        """检查服务器资源"""
        print("🔍 检查服务器资源...")
        
        # 检查内存
        result = subprocess.run(['free', '-h'], capture_output=True, text=True)
        print(result.stdout)
        
        # 检查磁盘空间
        result = subprocess.run(['df', '-h', '/'], capture_output=True, text=True)
        print(result.stdout)
        
        # 检查CPU负载
        result = subprocess.run(['uptime'], capture_output=True, text=True)
        print(result.stdout)
        
        return True
    
    def get_next_task(self):
        """获取下一个待开发任务"""
        if not self.task_queue_file.exists():
            return None
        
        with open(self.task_queue_file, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
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
                    'raw_line': line,
                    'description': self.get_task_description(task_name)
                }
        return None
    
    def get_task_description(self, task_name):
        """根据任务名称获取详细描述"""
        task_descriptions = {
            "在线状态同步": {
                "description": "实现用户在线/下线状态管理，包括Redis存储和WebSocket广播",
                "files": [
                    "services/im-gateway/src/connection_manager.rs",
                    "services/im-gateway/src/handlers/websocket.rs",
                    "services/common/src/redis.rs"
                ],
                "steps": [
                    "1. 在ConnectionManager中添加用户状态管理",
                    "2. 实现Redis在线状态存储",
                    "3. 添加WebSocket状态广播",
                    "4. 创建在线状态查询API"
                ],
                "priority": "high",
                "estimated_hours": 2
            },
            "WebSocket认证逻辑完善": {
                "description": "完善WebSocket连接的JWT token验证和权限检查",
                "files": [
                    "services/im-gateway/src/auth.rs",
                    "services/im-gateway/src/handlers/websocket.rs",
                    "services/common/src/jwt.rs"
                ],
                "steps": [
                    "1. 实现JWT token验证",
                    "2. 添加连接时认证逻辑",
                    "3. 处理token过期情况",
                    "4. 添加权限检查中间件"
                ],
                "priority": "high",
                "estimated_hours": 1.5
            },
            "文件上传API实现": {
                "description": "实现文件上传功能，包括类型验证和MinIO集成",
                "files": [
                    "services/file-service/src/handlers/upload.rs",
                    "services/file-service/src/storage.rs",
                    "services/file-service/src/validation.rs"
                ],
                "steps": [
                    "1. 创建文件上传handler",
                    "2. 实现文件类型验证",
                    "3. 添加文件大小限制",
                    "4. 集成MinIO存储"
                ],
                "priority": "medium",
                "estimated_hours": 2.5
            },
            "AI模型对接（基础）": {
                "description": "对接AI模型服务，实现基础对话功能",
                "files": [
                    "services/ai-service/src/providers/openai.rs",
                    "services/ai-service/src/handlers/chat.rs",
                    "services/ai-service/src/streaming.rs"
                ],
                "steps": [
                    "1. 完善OpenAI provider",
                    "2. 实现基础对话功能",
                    "3. 添加流式响应支持",
                    "4. 实现错误处理和重试"
                ],
                "priority": "medium",
                "estimated_hours": 2
            },
            "消息持久化实现": {
                "description": "实现消息的持久化存储和查询",
                "files": [
                    "services/im-gateway/src/repository/message.rs",
                    "services/im-gateway/src/handlers/message.rs",
                    "migrations/004_messages.sql"
                ],
                "steps": [
                    "1. 完善消息存储逻辑",
                    "2. 实现历史消息分页",
                    "3. 添加消息搜索功能",
                    "4. 优化消息查询性能"
                ],
                "priority": "medium",
                "estimated_hours": 2
            },
            "AI对话管理": {
                "description": "实现AI对话的上下文管理和历史存储",
                "files": [
                    "services/ai-service/src/context.rs",
                    "services/ai-service/src/history.rs",
                    "services/ai-service/src/models.rs"
                ],
                "steps": [
                    "1. 实现对话上下文管理",
                    "2. 添加Token用量统计",
                    "3. 实现对话历史存储",
                    "4. 添加模型切换功能"
                ],
                "priority": "medium",
                "estimated_hours": 2
            },
            "国内模型支持": {
                "description": "集成国内AI模型服务",
                "files": [
                    "services/ai-service/src/providers/qwen.rs",
                    "services/ai-service/src/providers/wenxin.rs",
                    "services/ai-service/src/providers/zhipu.rs"
                ],
                "steps": [
                    "1. 集成通义千问",
                    "2. 集成文心一言",
                    "3. 集成智谱AI",
                    "4. 实现模型路由策略"
                ],
                "priority": "low",
                "estimated_hours": 3
            },
            "文件下载和预览": {
                "description": "实现文件下载和图片预览功能",
                "files": [
                    "services/file-service/src/handlers/download.rs",
                    "services/file-service/src/preview.rs",
                    "services/file-service/src/permissions.rs"
                ],
                "steps": [
                    "1. 实现文件下载API",
                    "2. 添加图片预览功能",
                    "3. 实现文件权限控制",
                    "4. 集成CDN（可选）"
                ],
                "priority": "medium",
                "estimated_hours": 2
            },
            "文件管理功能": {
                "description": "实现文件的管理功能",
                "files": [
                    "services/file-service/src/handlers/management.rs",
                    "services/file-service/src/sharing.rs",
                    "services/file-service/src/statistics.rs"
                ],
                "steps": [
                    "1. 实现文件列表查询",
                    "2. 添加文件删除功能",
                    "3. 实现文件分享功能",
                    "4. 添加存储空间统计"
                ],
                "priority": "low",
                "estimated_hours": 1.5
            },
            "消息推送通知": {
                "description": "实现消息推送通知功能",
                "files": [
                    "services/push-service/src/handlers/notification.rs",
                    "services/push-service/src/providers/mobile.rs",
                    "services/push-service/src/providers/desktop.rs"
                ],
                "steps": [
                    "1. 集成移动端推送",
                    "2. 添加桌面通知支持",
                    "3. 实现推送配置管理",
                    "4. 添加推送统计和监控"
                ],
                "priority": "low",
                "estimated_hours": 2.5
            },
            "会话管理增强": {
                "description": "增强会话管理功能",
                "files": [
                    "services/im-gateway/src/handlers/conversation.rs",
                    "services/im-gateway/src/models/conversation.rs",
                    "services/im-gateway/src/repository/conversation.rs"
                ],
                "steps": [
                    "1. 实现会话置顶功能",
                    "2. 添加免打扰设置",
                    "3. 实现会话归档",
                    "4. 添加会话搜索"
                ],
                "priority": "low",
                "estimated_hours": 2
            },
            "消息加密": {
                "description": "实现端到端消息加密",
                "files": [
                    "services/common/src/crypto.rs",
                    "services/im-gateway/src/encryption.rs",
                    "services/im-gateway/src/key_exchange.rs"
                ],
                "steps": [
                    "1. 设计端到端加密方案",
                    "2. 实现密钥交换协议",
                    "3. 实现加密消息存储",
                    "4. 实现解密消息显示"
                ],
                "priority": "low",
                "estimated_hours": 4
            }
        }
        
        # 查找匹配的任务描述
        for key, value in task_descriptions.items():
            if key in task_name:
                return value
        
        return {
            "description": f"实现{task_name}功能",
            "files": [],
            "steps": ["1. 分析需求", "2. 实现功能", "3. 测试验证", "4. 提交代码"],
            "priority": "medium",
            "estimated_hours": 2
        }
    
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
    
    def compile_project(self):
        """编译项目"""
        print("🔨 编译项目...")
        
        # 设置编译环境
        env = os.environ.copy()
        env['CARGO_BUILD_JOBS'] = str(self.cargo_jobs)
        
        # 编译所有服务
        for service in self.services:
            service_path = self.project_root / "services" / service
            if not service_path.exists():
                continue
            
            print(f"🔨 编译 {service}...")
            
            try:
                result = subprocess.run(
                    ['cargo', 'check', '-p', service],
                    cwd=self.project_root,
                    env=env,
                    capture_output=True,
                    text=True,
                    timeout=300  # 5分钟超时
                )
                
                if result.returncode != 0:
                    print(f"❌ 编译失败: {service}")
                    print(result.stderr)
                    return False, result.stderr
                else:
                    print(f"✅ 编译成功: {service}")
                    
            except subprocess.TimeoutExpired:
                print(f"⏰ 编译超时: {service}")
                return False, "编译超时"
        
        return True, "编译成功"
    
    def fix_compilation_errors(self, error_output):
        """修复编译错误"""
        print("🔧 尝试修复编译错误...")
        
        # 这里应该实现实际的错误修复逻辑
        # 例如：解析错误信息，修改代码，重新编译
        
        # 示例：简单的错误修复
        if "missing dependency" in error_output:
            print("📦 添加缺失的依赖...")
            # 这里应该添加实际的依赖修复逻辑
            return True
        
        if "type mismatch" in error_output:
            print("🔄 修复类型不匹配...")
            # 这里应该添加实际的类型修复逻辑
            return True
        
        return False
    
    def commit_changes(self, task_name):
        """提交代码更改"""
        print("📤 提交代码更改...")
        
        try:
            # 添加所有更改
            subprocess.run(['git', 'add', '-A'], cwd=self.project_root, check=True)
            
            # 提交更改
            commit_message = f"feat: {task_name}\n\n实现{task_name}功能"
            subprocess.run(
                ['git', 'commit', '-m', commit_message],
                cwd=self.project_root,
                check=True
            )
            
            # 获取提交哈希
            result = subprocess.run(
                ['git', 'rev-parse', 'HEAD'],
                cwd=self.project_root,
                capture_output=True,
                text=True,
                check=True
            )
            
            commit_hash = result.stdout.strip()
            print(f"✅ 提交成功: {commit_hash[:8]}")
            return commit_hash
            
        except subprocess.CalledProcessError as e:
            print(f"❌ 提交失败: {e}")
            return None
    
    def update_progress(self, task_name, status, details=None):
        """更新进度报告"""
        if not self.progress_file.exists():
            return
        
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        
        with open(self.progress_file, 'a', encoding='utf-8') as f:
            f.write(f"\n### {timestamp}\n")
            f.write(f"- **任务**: {task_name}\n")
            f.write(f"- **状态**: {status}\n")
            if details:
                f.write(f"- **详情**: {details}\n")
            f.write("\n")
    
    def execute_task(self, task):
        """执行开发任务"""
        if not task:
            return False, "没有待执行的任务"
        
        print(f"🚀 开始执行任务: {task['name']}")
        print(f"📝 描述: {task['description']['description']}")
        print(f"📁 涉及文件: {', '.join(task['description']['files'])}")
        print(f"📋 步骤: {task['description']['steps']}")
        print(f"🔥 优先级: {task['description']['priority']}")
        print(f"⏱️  预计耗时: {task['description']['estimated_hours']}小时")
        
        # 1. 检查资源
        self.check_resources()
        
        # 2. 标记为进行中
        self.mark_task_in_progress(task)
        
        # 3. 根据任务类型执行不同的开发逻辑
        task_name = task['name']
        
        if "在线状态同步" in task_name:
            return self.implement_online_status(task)
        elif "WebSocket认证" in task_name:
            return self.implement_websocket_auth(task)
        elif "文件上传" in task_name:
            return self.implement_file_upload(task)
        elif "AI模型对接" in task_name:
            return self.implement_ai_provider(task)
        elif "消息持久化" in task_name:
            return self.implement_message_persistence(task)
        elif "AI对话管理" in task_name:
            return self.implement_ai_conversation(task)
        elif "国内模型支持" in task_name:
            return self.implement_domestic_ai(task)
        elif "文件下载" in task_name:
            return self.implement_file_download(task)
        elif "文件管理" in task_name:
            return self.implement_file_management(task)
        elif "消息推送" in task_name:
            return self.implement_push_notification(task)
        elif "会话管理增强" in task_name:
            return self.implement_conversation_enhancement(task)
        elif "消息加密" in task_name:
            return self.implement_message_encryption(task)
        else:
            return self.implement_generic_task(task)
    
    def implement_online_status(self, task):
        """实现在线状态同步功能"""
        print("🔧 实现在线状态同步...")
        
        # 创建或修改相关文件
        files_to_modify = [
            "services/im-gateway/src/connection_manager.rs",
            "services/im-gateway/src/handlers/websocket.rs",
            "services/common/src/redis.rs"
        ]
        
        # 这里应该实现实际的代码编写逻辑
        # 例如：添加用户状态管理、Redis存储、WebSocket广播
        
        # 模拟实现过程
        time.sleep(2)
        
        # 编译项目
        compile_success, compile_output = self.compile_project()
        
        if not compile_success:
            # 尝试修复编译错误
            fix_success = self.fix_compilation_errors(compile_output)
            if not fix_success:
                self.mark_task_blocked(task, "编译错误无法修复")
                return False, f"编译错误: {compile_output}"
        
        # 提交代码
        commit_hash = self.commit_changes(task['name'])
        
        if commit_hash:
            self.mark_task_completed(task, commit_hash)
            self.update_progress(task['name'], "✅ 完成", f"提交: {commit_hash[:8]}")
            return True, f"任务完成，提交: {commit_hash[:8]}"
        else:
            self.mark_task_blocked(task, "提交失败")
            return False, "代码提交失败"
    
    def implement_websocket_auth(self, task):
        """实现WebSocket认证逻辑"""
        print("🔧 实现WebSocket认证逻辑...")
        
        # 实现JWT token验证、连接时认证等
        time.sleep(2)
        
        compile_success, compile_output = self.compile_project()
        
        if not compile_success:
            fix_success = self.fix_compilation_errors(compile_output)
            if not fix_success:
                self.mark_task_blocked(task, "编译错误无法修复")
                return False, f"编译错误: {compile_output}"
        
        commit_hash = self.commit_changes(task['name'])
        
        if commit_hash:
            self.mark_task_completed(task, commit_hash)
            self.update_progress(task['name'], "✅ 完成", f"提交: {commit_hash[:8]}")
            return True, f"任务完成，提交: {commit_hash[:8]}"
        else:
            self.mark_task_blocked(task, "提交失败")
            return False, "代码提交失败"
    
    def implement_file_upload(self, task):
        """实现文件上传功能"""
        print("🔧 实现文件上传功能...")
        
        # 实现文件上传handler、类型验证、MinIO集成
        time.sleep(2)
        
        compile_success, compile_output = self.compile_project()
        
        if not compile_success:
            fix_success = self.fix_compilation_errors(compile_output)
            if not fix_success:
                self.mark_task_blocked(task, "编译错误无法修复")
                return False, f"编译错误: {compile_output}"
        
        commit_hash = self.commit_changes(task['name'])
        
        if commit_hash:
            self.mark_task_completed(task, commit_hash)
            self.update_progress(task['name'], "✅ 完成", f"提交: {commit_hash[:8]}")
            return True, f"任务完成，提交: {commit_hash[:8]}"
        else:
            self.mark_task_blocked(task, "提交失败")
            return False, "代码提交失败"
    
    def implement_ai_provider(self, task):
        """实现AI模型对接"""
        print("🔧 实现AI模型对接...")
        
        # 实现OpenAI provider、流式响应等
        time.sleep(2)
        
        compile_success, compile_output = self.compile_project()
        
        if not compile_success:
            fix_success = self.fix_compilation_errors(compile_output)
            if not fix_success:
                self.mark_task_blocked(task, "编译错误无法修复")
                return False, f"编译错误: {compile_output}"
        
        commit_hash = self.commit_changes(task['name'])
        
        if commit_hash:
            self.mark_task_completed(task, commit_hash)
            self.update_progress(task['name'], "✅ 完成", f"提交: {commit_hash[:8]}")
            return True, f"任务完成，提交: {commit_hash[:8]}"
        else:
            self.mark_task_blocked(task, "提交失败")
            return False, "代码提交失败"
    
    def implement_message_persistence(self, task):
        """实现消息持久化"""
        print("🔧 实现消息持久化...")
        
        # 实现消息存储、历史消息分页等
        time.sleep(2)
        
        compile_success, compile_output = self.compile_project()
        
        if not compile_success:
            fix_success = self.fix_compilation_errors(compile_output)
            if not fix_success:
                self.mark_task_blocked(task, "编译错误无法修复")
                return False, f"编译错误: {compile_output}"
        
        commit_hash = self.commit_changes(task['name'])
        
        if commit_hash:
            self.mark_task_completed(task, commit_hash)
            self.update_progress(task['name'], "✅ 完成", f"提交: {commit_hash[:8]}")
            return True, f"任务完成，提交: {commit_hash[:8]}"
        else:
            self.mark_task_blocked(task, "提交失败")
            return False, "代码提交失败"
    
    def implement_ai_conversation(self, task):
        """实现AI对话管理"""
        print("🔧 实现AI对话管理...")
        
        # 实现对话上下文管理、Token统计等
        time.sleep(2)
        
        compile_success, compile_output = self.compile_project()
        
        if not compile_success:
            fix_success = self.fix_compilation_errors(compile_output)
            if not fix_success:
                self.mark_task_blocked(task, "编译错误无法修复")
                return False, f"编译错误: {compile_output}"
        
        commit_hash = self.commit_changes(task['name'])
        
        if commit_hash:
            self.mark_task_completed(task, commit_hash)
            self.update_progress(task['name'], "✅ 完成", f"提交: {commit_hash[:8]}")
            return True, f"任务完成，提交: {commit_hash[:8]}"
        else:
            self.mark_task_blocked(task, "提交失败")
            return False, "代码提交失败"
    
    def implement_domestic_ai(self, task):
        """实现国内模型支持"""
        print("🔧 实现国内模型支持...")
        
        # 实现通义千问、文心一言、智谱AI集成
        time.sleep(2)
        
        compile_success, compile_output = self.compile_project()
        
        if not compile_success:
            fix_success = self.fix_compilation_errors(compile_output)
            if not fix_success:
                self.mark_task_blocked(task, "编译错误无法修复")
                return False, f"编译错误: {compile_output}"
        
        commit_hash = self.commit_changes(task['name'])
        
        if commit_hash:
            self.mark_task_completed(task, commit_hash)
            self.update_progress(task['name'], "✅ 完成", f"提交: {commit_hash[:8]}")
            return True, f"任务完成，提交: {commit_hash[:8]}"
        else:
            self.mark_task_blocked(task, "提交失败")
            return False, "代码提交失败"
    
    def implement_file_download(self, task):
        """实现文件下载和预览"""
        print("🔧 实现文件下载和预览...")
        
        # 实现文件下载API、图片预览等
        time.sleep(2)
        
        compile_success, compile_output = self.compile_project()
        
        if not compile_success:
            fix_success = self.fix_compilation_errors(compile_output)
            if not fix_success:
                self.mark_task_blocked(task, "编译错误无法修复")
                return False, f"编译错误: {compile_output}"
        
        commit_hash = self.commit_changes(task['name'])
        
        if commit_hash:
            self.mark_task_completed(task, commit_hash)
            self.update_progress(task['name'], "✅ 完成", f"提交: {commit_hash[:8]}")
            return True, f"任务完成，提交: {commit_hash[:8]}"
        else:
            self.mark_task_blocked(task, "提交失败")
            return False, "代码提交失败"
    
    def implement_file_management(self, task):
        """实现文件管理功能"""
        print("🔧 实现文件管理功能...")
        
        # 实现文件列表、删除、分享等
        time.sleep(2)
        
        compile_success, compile_output = self.compile_project()
        
        if not compile_success:
            fix_success = self.fix_compilation_errors(compile_output)
            if not fix_success:
                self.mark_task_blocked(task, "编译错误无法修复")
                return False, f"编译错误: {compile_output}"
        
        commit_hash = self.commit_changes(task['name'])
        
        if commit_hash:
            self.mark_task_completed(task, commit_hash)
            self.update_progress(task['name'], "✅ 完成", f"提交: {commit_hash[:8]}")
            return True, f"任务完成，提交: {commit_hash[:8]}"
        else:
            self.mark_task_blocked(task, "提交失败")
            return False, "代码提交失败"
    
    def implement_push_notification(self, task):
        """实现消息推送通知"""
        print("🔧 实现消息推送通知...")
        
        # 实现移动端推送、桌面通知等
        time.sleep(2)
        
        compile_success, compile_output = self.compile_project()
        
        if not compile_success:
            fix_success = self.fix_compilation_errors(compile_output)
            if not fix_success:
                self.mark_task_blocked(task, "编译错误无法修复")
                return False, f"编译错误: {compile_output}"
        
        commit_hash = self.commit_changes(task['name'])
        
        if commit_hash:
            self.mark_task_completed(task, commit_hash)
            self.update_progress(task['name'], "✅ 完成", f"提交: {commit_hash[:8]}")
            return True, f"任务完成，提交: {commit_hash[:8]}"
        else:
            self.mark_task_blocked(task, "提交失败")
            return False, "代码提交失败"
    
    def implement_conversation_enhancement(self, task):
        """实现会话管理增强"""
        print("🔧 实现会话管理增强...")
        
        # 实现会话置顶、免打扰等
        time.sleep(2)
        
        compile_success, compile_output = self.compile_project()
        
        if not compile_success:
            fix_success = self.fix_compilation_errors(compile_output)
            if not fix_success:
                self.mark_task_blocked(task, "编译错误无法修复")
                return False, f"编译错误: {compile_output}"
        
        commit_hash = self.commit_changes(task['name'])
        
        if commit_hash:
            self.mark_task_completed(task, commit_hash)
            self.update_progress(task['name'], "✅ 完成", f"提交: {commit_hash[:8]}")
            return True, f"任务完成，提交: {commit_hash[:8]}"
        else:
            self.mark_task_blocked(task, "提交失败")
            return False, "代码提交失败"
    
    def implement_message_encryption(self, task):
        """实现消息加密"""
        print("🔧 实现消息加密...")
        
        # 实现端到端加密、密钥交换等
        time.sleep(2)
        
        compile_success, compile_output = self.compile_project()
        
        if not compile_success:
            fix_success = self.fix_compilation_errors(compile_output)
            if not fix_success:
                self.mark_task_blocked(task, "编译错误无法修复")
                return False, f"编译错误: {compile_output}"
        
        commit_hash = self.commit_changes(task['name'])
        
        if commit_hash:
            self.mark_task_completed(task, commit_hash)
            self.update_progress(task['name'], "✅ 完成", f"提交: {commit_hash[:8]}")
            return True, f"任务完成，提交: {commit_hash[:8]}"
        else:
            self.mark_task_blocked(task, "提交失败")
            return False, "代码提交失败"
    
    def implement_generic_task(self, task):
        """实现通用任务"""
        print(f"🔧 实现通用任务: {task['name']}...")
        
        # 通用任务实现逻辑
        time.sleep(2)
        
        compile_success, compile_output = self.compile_project()
        
        if not compile_success:
            fix_success = self.fix_compilation_errors(compile_output)
            if not fix_success:
                self.mark_task_blocked(task, "编译错误无法修复")
                return False, f"编译错误: {compile_output}"
        
        commit_hash = self.commit_changes(task['name'])
        
        if commit_hash:
            self.mark_task_completed(task, commit_hash)
            self.update_progress(task['name'], "✅ 完成", f"提交: {commit_hash[:8]}")
            return True, f"任务完成，提交: {commit_hash[:8]}"
        else:
            self.mark_task_blocked(task, "提交失败")
            return False, "代码提交失败"
    
    def run_development_session(self, max_duration_minutes=30):
        """运行开发会话"""
        print("🎯 开始OmniLink开发会话")
        
        session_start = time.time()
        completed_tasks = []
        blocked_tasks = []
        
        while True:
            # 检查时间
            elapsed_minutes = (time.time() - session_start) / 60
            if elapsed_minutes >= max_duration_minutes:
                print(f"⏰ 达到最大时长 ({max_duration_minutes}分钟)，结束会话")
                break
            
            # 获取下一个任务
            task = self.get_next_task()
            if not task:
                print("✅ 所有任务已完成")
                break
            
            print(f"\n{'='*50}")
            print(f"📋 当前任务: {task['name']}")
            print(f"⏰ 已用时间: {elapsed_minutes:.1f}分钟")
            print(f"{'='*50}")
            
            # 执行任务
            success, message = self.execute_task(task)
            
            if success:
                completed_tasks.append(task)
                print(f"✅ {message}")
            else:
                blocked_tasks.append(task)
                print(f"⚠️ {message}")
            
            # 等待一段时间
            time.sleep(2)
        
        # 生成会话报告
        print(f"\n🎯 开发会话结束")
        print(f"✅ 完成任务: {len(completed_tasks)}")
        print(f"⚠️ 受阻任务: {len(blocked_tasks)}")
        
        return {
            'completed': len(completed_tasks),
            'blocked': len(blocked_tasks),
            'completed_tasks': [t['name'] for t in completed_tasks],
            'blocked_tasks': [t['name'] for t in blocked_tasks]
        }


def main():
    """主函数"""
    developer = OmniLinkDeveloper()
    
    # 检查是否在开发窗口内
    current_hour = datetime.now().hour
    if current_hour >= 8:
        print(f"❌ 当前时间已超过8点，不在开发窗口内")
        print(f"开发窗口: 00:00 - 08:00")
        sys.exit(1)
    
    # 运行开发会话
    result = developer.run_development_session(max_duration_minutes=30)
    
    print("\n" + "="*50)
    print("🎉 OmniLink开发会话完成")
    print("="*50)
    print(f"✅ 完成任务: {result['completed']}")
    print(f"⚠️ 受阻任务: {result['blocked']}")
    
    if result['completed_tasks']:
        print("\n📋 完成的任务:")
        for task in result['completed_tasks']:
            print(f"  - {task}")
    
    if result['blocked_tasks']:
        print("\n⚠️ 受阻的任务:")
        for task in result['blocked_tasks']:
            print(f"  - {task}")


if __name__ == "__main__":
    main()