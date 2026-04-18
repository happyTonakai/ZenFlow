# ZenFlow - AI Paper & News Recommendation Agent

set shell := ["bash", "-c"]

export PATH := env("HOME") + "/.cargo/bin:" + env("PATH")

# 默认目录
rust_dir := "src-tauri/src-tauri"
frontend_dir := "src-tauri"

# 默认任务：显示帮助
default:
    @just --list

# ============ 开发 ============

# 启动 Tauri 开发模式（前端 + 后端热重载）
dev:
    cd {{frontend_dir}} && npm run tauri dev

# 启动前端开发服务器（仅 Vite）
dev-frontend:
    cd {{frontend_dir}} && npm run dev

# ============ 构建 ============

# 完整构建（前端 + Rust）
build: build-frontend build-rust

# 构建前端
build-frontend:
    cd {{frontend_dir}} && npm run build

# 构建 Rust 后端
build-rust:
    cd {{rust_dir}} && cargo build

# Rust release 构建
build-release:
    cd {{rust_dir}} && cargo build --release

# 类型检查（不生成产物）
check: check-rust check-ts

# Rust 类型检查
check-rust:
    cd {{rust_dir}} && cargo check

# TypeScript 类型检查
check-ts:
    cd {{frontend_dir}} && npx tsc --noEmit

# ============ 测试 ============

# 运行所有单元测试
test:
    cd {{rust_dir}} && cargo test --lib

# 运行 LLM 集成测试（需要本地 Ollama）
test-llm:
    cd {{rust_dir}} && cargo test -- --ignored --test-threads=1

# 运行全部测试（单元 + 集成）
test-all:
    cd {{rust_dir}} && cargo test -- --include-ignored --test-threads=1

# ============ 清理 ============

# 清理 Rust 构建产物
clean:
    cd {{rust_dir}} && cargo clean

# 清理全部（Rust + node_modules）
clean-all: clean
    rm -rf {{frontend_dir}}/node_modules {{frontend_dir}}/dist

# ============ 依赖 ============

# 安装全部依赖
install:
    cd {{frontend_dir}} && npm install

# 更新 Rust 依赖
update-rust:
    cd {{rust_dir}} && cargo update

# ============ 工具 ============

# Rust clippy 检查
clippy:
    cd {{rust_dir}} && cargo clippy -- -W warnings

# 打包 Tauri 应用
bundle:
    cd {{frontend_dir}} && npm run tauri build
