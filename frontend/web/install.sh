#!/bin/bash

# OmniLink Web 前端依赖安装脚本

echo "📦 安装 OmniLink Web 前端依赖..."
echo ""

# 使用国内镜像加速
export NPM_REGISTRY=https://registry.npmmirror.com

# 方法1: 使用 npm
echo "方法1: 使用 npm 安装"
npm install --registry=$NPM_REGISTRY

# 如果 npm 安装失败，尝试方法2: 使用 yarn
if [ $? -ne 0 ]; then
    echo ""
    echo "npm 安装失败，尝试使用 yarn..."
    yarn install --registry=$NPM_REGISTRY
fi

# 如果 yarn 安装失败，尝试方法3: 使用 pnpm
if [ $? -ne 0 ]; then
    echo ""
    echo "yarn 安装失败，尝试使用 pnpm..."
    pnpm install --registry=$NPM_REGISTRY
fi

echo ""
echo "✅ 依赖安装完成！"
echo ""
echo "启动开发服务器:"
echo "  npm run dev"
echo "  或"
echo "  yarn dev"
echo "  或"
echo "  pnpm dev"
