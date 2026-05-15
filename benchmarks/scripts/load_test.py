#!/usr/bin/env python3
"""
OmniLink 负载测试脚本

使用 asyncio 和 aiohttp 进行高并发负载测试

使用方法:
    python3 load_test.py --users 100 --duration 60 --ramp-up 10

环境变量:
    OMNILINK_URL: 服务地址 (默认: http://localhost:8080)
    AUTH_TOKEN: 认证令牌 (必需)
"""

import asyncio
import aiohttp
import argparse
import json
import time
import uuid
import statistics
from dataclasses import dataclass, field
from typing import List, Dict
from datetime import datetime


@dataclass
class TestResult:
    """测试结果"""
    total_requests: int = 0
    successful_requests: int = 0
    failed_requests: int = 0
    response_times: List[float] = field(default_factory=list)
    errors: Dict[str, int] = field(default_factory=dict)
    start_time: float = 0
    end_time: float = 0

    @property
    def duration(self) -> float:
        return self.end_time - self.start_time

    @property
    def requests_per_second(self) -> float:
        if self.duration == 0:
            return 0
        return self.total_requests / self.duration

    @property
    def avg_response_time(self) -> float:
        if not self.response_times:
            return 0
        return statistics.mean(self.response_times)

    @property
    def p95_response_time(self) -> float:
        if not self.response_times:
            return 0
        sorted_times = sorted(self.response_times)
        index = int(len(sorted_times) * 0.95)
        return sorted_times[index]

    @property
    def p99_response_time(self) -> float:
        if not self.response_times:
            return 0
        sorted_times = sorted(self.response_times)
        index = int(len(sorted_times) * 0.99)
        return sorted_times[index]


class LoadTester:
    """负载测试器"""

    def __init__(self, base_url: str, auth_token: str):
        self.base_url = base_url.rstrip('/')
        self.auth_token = auth_token
        self.headers = {
            'Authorization': f'Bearer {auth_token}',
            'Content-Type': 'application/json'
        }
        self.result = TestResult()
        self._stop_event = asyncio.Event()

    async def send_message(self, session: aiohttp.ClientSession, conversation_id: str) -> bool:
        """发送消息"""
        payload = {
            'conversationId': conversation_id,
            'content': f'负载测试消息 {uuid.uuid4().hex[:8]}',
            'contentType': 'text',
            'metadata': {}
        }

        start = time.time()
        try:
            async with session.post(
                f'{self.base_url}/api/im/messages',
                headers=self.headers,
                json=payload,
                timeout=aiohttp.ClientTimeout(total=10)
            ) as response:
                elapsed = time.time() - start
                self.result.response_times.append(elapsed)
                self.result.total_requests += 1

                if response.status == 200:
                    self.result.successful_requests += 1
                    return True
                else:
                    self.result.failed_requests += 1
                    error_key = f'HTTP_{response.status}'
                    self.result.errors[error_key] = self.result.errors.get(error_key, 0) + 1
                    return False
        except Exception as e:
            elapsed = time.time() - start
            self.result.response_times.append(elapsed)
            self.result.total_requests += 1
            self.result.failed_requests += 1
            error_key = type(e).__name__
            self.result.errors[error_key] = self.result.errors.get(error_key, 0) + 1
            return False

    async def get_conversations(self, session: aiohttp.ClientSession) -> bool:
        """获取会话列表"""
        start = time.time()
        try:
            async with session.get(
                f'{self.base_url}/api/im/conversations',
                headers=self.headers,
                timeout=aiohttp.ClientTimeout(total=10)
            ) as response:
                elapsed = time.time() - start
                self.result.response_times.append(elapsed)
                self.result.total_requests += 1

                if response.status == 200:
                    self.result.successful_requests += 1
                    return True
                else:
                    self.result.failed_requests += 1
                    return False
        except Exception as e:
            elapsed = time.time() - start
            self.result.response_times.append(elapsed)
            self.result.total_requests += 1
            self.result.failed_requests += 1
            error_key = type(e).__name__
            self.result.errors[error_key] = self.result.errors.get(error_key, 0) + 1
            return False

    async def user_simulation(
        self,
        user_id: int,
        duration: int,
        conversation_id: str
    ):
        """模拟单个用户行为"""
        async with aiohttp.ClientSession() as session:
            end_time = time.time() + duration

            while time.time() < end_time and not self._stop_event.is_set():
                # 80% 概率发送消息，20% 概率查询
                if uuid.uuid4().int % 5 == 0:
                    await self.get_conversations(session)
                else:
                    await self.send_message(session, conversation_id)

                # 随机延迟 100-500ms
                await asyncio.sleep(0.1 + uuid.uuid4().int % 400 / 1000)

    async def run(
        self,
        num_users: int,
        duration: int,
        ramp_up: int
    ):
        """运行负载测试"""
        print(f'\n{"="*60}')
        print(f'OmniLink 负载测试')
        print(f'{"="*60}')
        print(f'目标地址: {self.base_url}')
        print(f'并发用户: {num_users}')
        print(f'测试时长: {duration} 秒')
        print(f'爬坡时间: {ramp_up} 秒')
        print(f'开始时间: {datetime.now().strftime("%Y-%m-%d %H:%M:%S")}')
        print(f'{"="*60}\n')

        conversation_id = str(uuid.uuid4())
        self.result = TestResult()
        self.result.start_time = time.time()

        # 分批启动用户
        users_per_batch = max(1, num_users // ramp_up)
        tasks = []

        for i in range(num_users):
            task = asyncio.create_task(
                self.user_simulation(i, duration, conversation_id)
            )
            tasks.append(task)

            # 控制启动速率
            if (i + 1) % users_per_batch == 0:
                await asyncio.sleep(1)
                print(f'已启动 {i + 1}/{num_users} 用户')

        print(f'\n所有用户已启动，测试进行中...\n')

        # 等待所有任务完成
        await asyncio.gather(*tasks)

        self.result.end_time = time.time()

        # 打印结果
        self.print_results()

    def print_results(self):
        """打印测试结果"""
        print(f'\n{"="*60}')
        print(f'测试结果')
        print(f'{"="*60}')
        print(f'测试时长: {self.result.duration:.2f} 秒')
        print(f'总请求数: {self.result.total_requests}')
        print(f'成功请求: {self.result.successful_requests}')
        print(f'失败请求: {self.result.failed_requests}')
        print(f'成功率: {self.result.successful_requests / max(1, self.result.total_requests) * 100:.2f}%')
        print(f'')
        print(f'吞吐量: {self.result.requests_per_second:.2f} req/s')
        print(f'')
        print(f'响应时间:')
        print(f'  平均: {self.result.avg_response_time * 1000:.2f} ms')
        print(f'  P95: {self.result.p95_response_time * 1000:.2f} ms')
        print(f'  P99: {self.result.p99_response_time * 1000:.2f} ms')
        print(f'  最小: {min(self.result.response_times) * 1000:.2f} ms')
        print(f'  最大: {max(self.result.response_times) * 1000:.2f} ms')

        if self.result.errors:
            print(f'\n错误统计:')
            for error, count in self.result.errors.items():
                print(f'  {error}: {count}')

        print(f'\n{"="*60}')

        # 生成报告文件
        self.save_report()

    def save_report(self):
        """保存测试报告"""
        report = {
            'timestamp': datetime.now().isoformat(),
            'config': {
                'base_url': self.base_url,
            },
            'results': {
                'duration': self.result.duration,
                'total_requests': self.result.total_requests,
                'successful_requests': self.result.successful_requests,
                'failed_requests': self.result.failed_requests,
                'requests_per_second': self.result.requests_per_second,
                'response_times': {
                    'avg_ms': self.result.avg_response_time * 1000,
                    'p95_ms': self.result.p95_response_time * 1000,
                    'p99_ms': self.result.p99_response_time * 1000,
                    'min_ms': min(self.result.response_times) * 1000,
                    'max_ms': max(self.result.response_times) * 1000,
                },
                'errors': self.result.errors
            }
        }

        filename = f'load_test_{datetime.now().strftime("%Y%m%d_%H%M%S")}.json'
        with open(filename, 'w') as f:
            json.dump(report, f, indent=2, ensure_ascii=False)
        print(f'\n详细报告已保存: {filename}')


async def main():
    parser = argparse.ArgumentParser(description='OmniLink 负载测试工具')
    parser.add_argument('--users', type=int, default=50, help='并发用户数 (默认: 50)')
    parser.add_argument('--duration', type=int, default=60, help='测试时长/秒 (默认: 60)')
    parser.add_argument('--ramp-up', type=int, default=10, help='爬坡时间/秒 (默认: 10)')
    parser.add_argument('--url', type=str, help='服务地址')
    parser.add_argument('--token', type=str, help='认证令牌')

    args = parser.parse_args()

    import os
    base_url = args.url or os.getenv('OMNILINK_URL', 'http://localhost:8080')
    auth_token = args.token or os.getenv('AUTH_TOKEN')

    if not auth_token:
        print('错误: 请设置 AUTH_TOKEN 环境变量或使用 --token 参数')
        return

    tester = LoadTester(base_url, auth_token)
    await tester.run(args.users, args.duration, args.ramp_up)


if __name__ == '__main__':
    asyncio.run(main())
