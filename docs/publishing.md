# 发布流程

hwpush 发布到 [crates.io](https://crates.io/crates/hwpush)。发布前必须先通过 `skill-check` 确认与 `today-task` skill 的格式兼容性。

## 前置条件

- 已登录 crates.io：`cargo login`（token 保存在 `~/.cargo/credentials`）
- 本地与远端 `master` 同步
- 对仓库有 push 权限（打 tag 用）

## 发布步骤

### 1. 检查 skill 兼容性

```bash
cargo build --release
./target/release/hwpush skill-check
```

- 输出 `✅ 已是最新版本` 或 `✅ 兼容` → 继续
- 输出 `🚨 检测到需要关注的变更` → 先同步负载格式（`src/core/pusher.rs`、`src/core/validator.rs`、`src/config/profile.rs`），再执行 `hwpush skill-check --mark-synced`

### 2. 运行测试

```bash
cargo test
cargo clippy
cargo fmt --check
```

### 3. bump 版本

按语义化版本（SemVer）修改 `Cargo.toml` 的 `version`：

- **patch**（0.1.1 → 0.1.2）：修复 Bug、微调
- **minor**（0.1.x → 0.2.0）：新增功能（如新命令）
- **major**（0.x → 1.0）：破坏性变更

同时更新 README 中的示例输出（如有版本相关描述）。

### 4. 打包校验

```bash
cargo package
cargo package --list   # 检查 .crate 包含的文件
```

`cargo package` 会校验 README 渲染、依赖、许可证；发现问题先修复再继续。

### 5. 发布

```bash
cargo publish
```

发布成功后验证：`cargo install hwpush`（或用 `cargo search hwpush` 确认新版本可见）。

### 6. 打 tag 并推送

```bash
git tag v0.2.0          # 与 Cargo.toml 版本一致
git push origin v0.2.0
```

## 回滚（必要时）

crates.io 不允许删除版本，只能 yank（标记不可用）：

```bash
cargo yank --version 0.2.0
```

已发布版本仍可下载，但新安装会跳过该版本。

## 注意事项

- **依赖变更**：新增依赖需同时更新 `Cargo.toml` 与 `cargo package` 产物确认
- **一次性发布**：crates.io 版本号不可复用，发布前确认版本号无误
- **认证码**：发布包中不得包含任何认证码或密钥（`cargo package --list` 检查）
