# 发布指南

## 自动构建工作流

本项目配置了 GitHub Actions 自动构建工作流，支持 Windows 平台的自动构建和发布。

### 工作流说明

#### 1. Release 工作流 (`.github/workflows/release.yml`)
- **触发条件**: 推送以 `v` 开头的标签（如 `v1.0.0`）或手动触发
- **功能**: 
  - 构建 Windows x64  release 版本
  - 打包资源文件和配置文件
  - 创建 GitHub Release 并上传构建产物
  - 使用推送的标签作为发布版本号

#### 2. Build 工作流 (`.github/workflows/build.yml`)
- **触发条件**: 
  - 推送到 `main` 或 `dev` 分支
  - 提交 Pull Request 到 `main` 分支
  - 手动触发
- **功能**:
  - 构建 debug 和 release 版本
  - 上传构建产物作为 artifact

### 如何使用

#### 创建发布版本

1. **创建并推送标签**:
   ```bash
   git tag v1.0.0
   git push origin v1.0.0
   ```

2. **GitHub Actions 会自动**:
   - 使用你推送的标签（如 v1.0.0）作为发布版本
   - 构建 Windows x64 release 版本
   - 打包所有必要文件（包括 res 目录、config.toml 等）
   - 创建 GitHub Release
   - 上传 ZIP 压缩包作为 release 资产

#### 手动测试构建

1. 进入 GitHub 仓库的 Actions 页面
2. 选择 "Build Windows" 工作流
3. 点击 "Run workflow" 手动触发构建

### 构建产物

- **Release 构建**: `classpaper-windows-x64.zip`
  - 包含：
    - `classpaper.exe` (主程序)
    - `res/` (资源文件目录)
    - `config.toml` (配置文件)
    - `README.md` 和 `LICENSE`

- **测试构建**: 
  - `classpaper-windows-x64-debug.exe`
  - `classpaper-windows-x64-release.exe`

### 注意事项

1. 确保 `Cargo.lock` 文件已提交到版本控制
2. 所有资源文件（res 目录）都已正确提交
3. 配置文件 `config.toml` 存在且格式正确
4. 版本标签格式应为 `v*.*.*`（如 v1.0.0）

### 依赖缓存

工作流使用 GitHub Actions 的缓存机制来加速 Rust 依赖的下载和编译过程。缓存键基于 `Cargo.lock` 文件的哈希值。