# 🧠 MiMo-TUI

> 一个基于 Rust 构建的终端原生智能编程助手，以 **MiMo V2.5 Pro** 为核心模型，充分利用 1M Token 超长上下文窗口。支持 TUI 终端界面和 CLI 命令行两种交互方式，无需 Node.js 或 Python 运行时。

---

## 目录

1. [项目简介](#项目简介)
2. [功能特性](#功能特性)
3. [环境要求](#环境要求)
4. [安装与运行](#安装与运行)
5. [配置说明](#配置说明)
6. [TUI 使用指南](#tui-使用指南)
7. [CLI 使用指南](#cli-使用指南)
8. [命令参考](#命令参考)
9. [记忆系统](#记忆系统)
10. [工具系统](#工具系统)
11. [SubAgent 系统](#subagent-系统)
12. [Provider 支持](#provider-支持)
13. [项目结构](#项目结构)
14. [开发指南](#开发指南)

---

## 项目简介

MiMo-TUI 是一个运行在终端中的智能编程助手，让 MiMo 模型可以直接访问你的工作区——读写文件、执行 Shell 命令、搜索网络、管理 Git，以及编排子智能体并行工作。

**核心设计理念**：

- **MiMo-First**：以小米 MiMo V2.5 Pro 为核心模型，充分利用其 1M Token 上下文窗口和长程稳定性
- **双界面支持**：同时提供 TUI（终端交互界面）和 CLI（命令行）两种使用方式
- **共享核心**：TUI 和 CLI 共用同一套核心引擎，仅在交互层有差异
- **模块化设计**：支持自定义工具和第三方扩展

---

## 功能特性

- **🗣️ 多轮对话** — 完整的对话历史管理，支持流式响应和打字机效果
- **🧠 思考过程可视化** — 实时展示模型的推理思考过程（Thinking Block）
- **🔧 12 个内置工具** — 文件读写、Shell 执行、代码搜索、Web 搜索、Git 状态等
- **📝 持久记忆系统** — 跨会话的智能体记忆和用户画像，自动注入系统提示
- **🤖 SubAgent 并行编排** — 7 种角色类型的子智能体，支持并行任务执行
- **💾 会话持久化** — 自动保存和恢复会话，支持检查点（Checkpoint）
- **🛡️ 工具审批机制** — 三层风险等级（Low/Medium/High），支持批准/拒绝/全部批准
- **🔄 多 Provider 支持** — 支持 MiMo、DeepSeek、OpenAI、OpenRouter
- **⌨️ 全键盘操作** — Vim 风格快捷键，完全键盘驱动
- **📊 分屏视图** — Chat / SubAgent / History 三视图切换

---

## 环境要求

- **Rust** 1.85+（用于从源码编译）
- **操作系统**：macOS、Linux、Windows
- **API Key**：需要 [MiMo API Key](https://api.xiaomimimo.com) 或 [DeepSeek API Key](https://platform.deepseek.com)

---

## 安装与运行

### 从源码编译

```bash
# 克隆仓库
git clone https://github.com/mimo/mimo-tui.git
cd mimo-tui

# 编译（首次编译需要下载依赖，可能需要几分钟）
cargo build --release

# 运行 TUI 版本
export MIMO_API_KEY="你的API密钥"
cargo run --bin mimo-tui

# 运行 CLI 版本
cargo run --bin mimo
```

### 快速启动

```bash
# 设置 API Key
export MIMO_API_KEY="sk-your-api-key-here"

# 启动 TUI
cargo run --bin mimo-tui

# 或启动 CLI
cargo run --bin mimo
```

---

## 配置说明

### 环境变量

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| `MIMO_API_KEY` | MiMo API 访问密钥（必填） | - |
| `DEEPSEEK_API_KEY` | DeepSeek API 访问密钥 | - |
| `MIMO_PROVIDER` | Provider 类型：`mimo` / `deepseek` | `mimo` |
| `MIMO_MODEL` | 使用的模型 ID | `mimo-v2.5-pro` |
| `MIMO_MEMORY` | 启用记忆系统（设置为任意值即可） | 未启用 |

### 配置文件

配置文件位于 `~/.mimo/config.toml`。将 API Key 写入配置文件后，**无需每次启动时设置环境变量**，程序会自动读取。

#### 一键创建

```bash
mkdir -p ~/.mimo
cat > ~/.mimo/config.toml << 'EOF'
provider = "mimo"
model = "mimo-v2.5-pro"
api_key = "sk-your-api-key"
EOF
```

#### 完整配置示例

```toml
provider = "mimo"
model = "mimo-v2.5-pro"
api_key = "sk-your-api-key"

# 沙箱模式: off / on / auto
sandbox_mode = "auto"

# 审批策略: ask / auto_approve / never
approval_policy = "ask"
```

#### DeepSeek 配置示例

```toml
provider = "deepseek"
model = "deepseek-v4-pro"
api_key = "sk-your-deepseek-key"
```

#### 优先级说明

程序读取 API Key 的优先级从高到低为：

1. **环境变量** `MIMO_API_KEY` / `DEEPSEEK_API_KEY`（最高）
2. **配置文件** `~/.mimo/config.toml` 中的 `api_key` 字段
3. **默认值**（空）

> ⚠️ **安全提示**：配置文件中的 API Key 是明文存储的。建议将 `~/.mimo/` 目录权限设为 `700`（`chmod 700 ~/.mimo`），切勿将配置文件提交到 Git 仓库。

### 数据存储

所有数据存储在 `~/.mimo/` 目录下：

```
~/.mimo/
├── config.toml         # 配置文件
├── sessions/           # 会话数据（JSON 格式）
│   ├── <session-id>.json
│   └── checkpoints/    # 会话检查点
├── memories/           # 记忆存储
│   ├── MEMORY.md       # Agent 记忆笔记
│   └── USER.md         # 用户画像
```

---

## TUI 使用指南

### 界面布局

```
┌──────────────────────────────────────────────┐
│ MiMo-TUI | mimo-v2.5-pro        [Chat] [M:5] │  ← 标题栏
├──────────────────────────────────────────────┤
│                                              │
│  ┌ You                                       │
│  │ 用户的输入消息...                          │
│                                              │
│  ┌ MiMo                                      │
│  │ ┌─ Thinking ──────────────────────┐        │
│  │ │ 模型的推理思考过程...             │        │
│  │ └────────────────────────────────┘        │
│  │ 模型的回复内容...                          │
│  │ ✓ Tool: file_read                         │
│  │    Result: 文件内容...                     │
│                                              │
├──────────────────────────────────────────────┤
│ > _                                          │  ← 输入区域
├──────────────────────────────────────────────┤
│ [Tab] Chat | Agents | History  [i] Input  [q] Quit  ← 状态栏
└──────────────────────────────────────────────┘
```

### 键盘快捷键

#### 全局快捷键

| 快捷键 | 功能 |
|--------|------|
| `q` | 退出程序 |
| `Ctrl + C` | 正在流式输出时取消，否则退出 |
| `?` | 打开帮助弹窗 |
| `Esc` | 取消当前操作 / 关闭弹窗 |

#### 视图切换

| 快捷键 | 功能 |
|--------|------|
| `Tab` | 切换到下一个视图（Chat → SubAgents → History → Chat） |
| `Shift + Tab` | 切换到上一个视图 |

#### 输入模式

| 快捷键 | 功能 |
|--------|------|
| `i` | 进入插入模式（开始输入消息） |
| `Ctrl + Enter` | 发送消息 |
| `Esc` | 取消输入 |

#### 命令模式

| 快捷键 | 功能 |
|--------|------|
| `/` 或 `:` | 进入命令模式 |
| `Enter` | 执行命令 |
| `Esc` | 取消命令 |
| `Backspace` | 删除命令字符 |

#### Chat 视图滚动

| 快捷键 | 功能 |
|--------|------|
| `j` / `↓` | 向下滚动 5 行 |
| `k` / `↑` | 向上滚动 5 行 |
| `Ctrl + d` | 向下翻半页（15 行） |
| `Ctrl + u` | 向上翻半页（15 行） |
| `g` | 滚动到顶部 |
| `G` | 滚动到底部 |

#### 审批模式

| 快捷键 | 功能 |
|--------|------|
| `a` | 批准当前工具调用 |
| `r` | 拒绝当前工具调用 |
| `A` | 批准本次会话所有后续工具调用 |
| `q` / `Esc` | 取消（等同于拒绝） |

---

## CLI 使用指南

CLI 版本提供纯粹的终端命令行交互：

```bash
$ export MIMO_API_KEY="sk-your-key"
$ cargo run --bin mimo

═══════════════════════════════════
  MiMo CLI - Intelligent Assistant
  Provider: mimo
  Model: mimo-v2.5-pro
  Memory: disabled
═══════════════════════════════════

[You]> 帮我读取 README.md 文件

MiMo: README.md 文件的内容如下：
...

[You]> # 这是一个重要的项目笔记   ← 以 # 开头自动保存为记忆

✓ Quick memory added (15% used)

[You]> /help                         ← 斜杠命令

Available Commands:
  /quit        - 退出程序
  /help        - 显示帮助
  /clear       - 清屏并开始新会话
  /memory      - 记忆管理 (show / add <note> / clear / path / help)
  /provider    - 切换 Provider (mimo / deepseek)
  /model       - 切换模型
  /session     - 会话管理 (list / new / switch <id> / save)
  /approval    - 审批策略设置 (ask / auto / never)
```

### 快捷记忆输入

在 CLI 中，以 `#` 开头且**不包含换行符**的单行输入会自动保存为 Agent 记忆，不触发 API 调用：

```bash
[You]> # 项目使用 Rust 2024 edition，ratatui 0.29 作为 TUI 框架
✓ Quick memory added (20% used)
```

---

## 命令参考

### `/help` — 显示帮助

打开帮助弹窗（TUI）或显示命令列表（CLI）。

### `/clear` — 清屏

清空当前对话并创建新会话。

### `/quit` — 退出

退出程序。

### `/memory` — 记忆管理

| 子命令 | 说明 | 示例 |
|--------|------|------|
| `show` | 显示所有记忆条目 | `/memory show` |
| `add <内容>` | 添加记忆条目 | `/memory add 项目使用 Rust 2024` |
| `clear` | 提示清空记忆的方法 | `/memory clear` |
| `path` | 显示记忆文件路径 | `/memory path` |
| `help` | 显示记忆帮助 | `/memory help` |

### `/provider <name>` — 切换 Provider

```bash
/provider mimo       # 切换到 MiMo
/provider deepseek   # 切换到 DeepSeek
```

### `/model <id>` — 切换模型

```bash
/model mimo-v2.5-pro
/model mimo-v2.5-flash
/model deepseek-v4-pro
```

### `/session` — 会话管理

| 子命令 | 说明 |
|--------|------|
| `list` | 列出所有保存的会话 |
| `new` | 创建新会话 |
| `switch <id>` | 切换到指定会话 |
| `save` | 保存当前会话 |

### `/approval <policy>` — 审批策略

| 策略 | 说明 |
|------|------|
| `ask` | 每次工具调用都询问（默认） |
| `auto` | 自动批准低风险工具 |
| `never` | 从不审批（全部自动执行） |

### `/compact` — 压缩上下文

当对话历史过长时，触发上下文压缩以释放 Token。

### `/tools` — 查看工具列表

显示当前可用的所有工具。

### `/sessions` — 查看会话历史

切换到 History 视图（TUI 中）。

---

## 记忆系统

MiMo-TUI 内置了一个持久化的记忆系统，让智能体可以在多次对话中持续学习和记住重要信息。

### 两种记忆目标

| 目标 | 说明 | 容量上限 | 文件位置 |
|------|------|----------|----------|
| **Memory** | Agent 的工作笔记，用于记录项目信息、偏好等 | 3000 字符 | `~/.mimo/memories/MEMORY.md` |
| **User** | 用户画像，记录用户偏好和行为模式 | 1500 字符 | `~/.mimo/memories/USER.md` |

### 添加记忆的方式

1. **快捷输入**（CLI 中）：以 `#` 开头的单行输入
2. **命令**：`/memory add <内容>`
3. **AI 自动调用**：智能体可以主动调用 `remember` 工具保存记忆

### 记忆注入

每次发送消息时，记忆内容会以系统提示块的形式自动注入到上下文中：

```
[Memory — 15% used]
§ 项目使用 ratatui 0.29 作为 TUI 框架
§ 用户偏好中文回复
```

### 记忆快照

系统在初始化时会自动拍摄记忆快照，确保在整个会话期间记忆的一致性。

---

## 工具系统

MiMo 内置了 12 个工具，分为 6 个类别：

### 文件操作

| 工具 | 说明 | 权限要求 |
|------|------|----------|
| `file_read` | 读取文件内容 | 无 |
| `file_write` | 写入/创建文件 | `file_write` |

### 搜索工具

| 工具 | 说明 | 权限要求 |
|------|------|----------|
| `grep` | 正则表达式搜索代码 | 无 |
| `glob` | 文件名模式匹配查找 | 无 |

### Shell 工具

| 工具 | 说明 | 权限要求 |
|------|------|----------|
| `shell` | 执行 Shell 命令 | `shell` |

### Git 工具

| 工具 | 说明 | 权限要求 |
|------|------|----------|
| `git_status` | 查看 Git 仓库状态 | 无 |

### 网络工具

| 工具 | 说明 | 权限要求 |
|------|------|----------|
| `web_fetch` | 抓取 URL 内容 | `network` |
| `web_search` | 网页搜索（基于 DuckDuckGo） | `network` |

### 记忆工具

| 工具 | 说明 | 权限要求 |
|------|------|----------|
| `remember` | 保存记忆笔记 | 无 |
| `memory_replace` | 替换已有记忆条目 | `memory_write` |
| `memory_remove` | 删除记忆条目 | `memory_write` |
| `recall_archive` | BM25 搜索历史对话档案 | 无 |

### 工具审批

部分高风险工具在执行前需要用户审批：

| 风险等级 | 判定条件 | 示例 |
|----------|----------|------|
| 🔴 High | 包含 `rm -rf`、`sudo`、`delete` | `rm -rf ./node_modules` |
| 🟡 Medium | 包含 `write`、`> `、`mv` | `echo "data" > config.json` |
| 🟢 Low | 其他命令 | `git status` |

---

## SubAgent 系统

SubAgent（子智能体）系统允许将复杂任务拆分为多个并行的子任务，每个子智能体有独立的角色和权限。

### 7 种子智能体角色

| 角色 | 说明 | 权限 |
|------|------|------|
| **General** | 全能型智能体 | 完整文件读写、Shell |
| **Explore** | 代码探索者 | 只读，信息收集 |
| **Plan** | 规划者 | 可写计划文件 |
| **Review** | 代码审查者 | 完全只读 |
| **Implementer** | 代码实现者 | 全部权限 |
| **Verifier** | 验证者 | 执行测试权限 |
| **Custom** | 自定义 | 白名单控制 |

### 状态流转

```
Pending → Running → Completed
                 → Failed
                 → Cancelled
                 → Interrupted → Resumed
```

### 使用方式

在 TUI 中按 `Tab` 切换到 **SubAgents** 视图，可以查看所有子智能体的运行状态和详情。

---

## Provider 支持

### MiMo

```bash
export MIMO_API_KEY="sk-your-key"
export MIMO_PROVIDER="mimo"
```

**默认模型**：`mimo-v2.5-pro`

**可用模型**：
- `mimo-v2.5-pro` — 支持工具调用和推理思考
- `mimo-v2.5-flash` — 快速版本，同样支持工具和推理

**API 端点**：`https://api.xiaomimimo.com/v1`

### DeepSeek

```bash
export DEEPSEEK_API_KEY="sk-your-key"
export MIMO_PROVIDER="deepseek"
```

**可用模型**：
- `deepseek-v4-pro` — 支持工具调用和推理思考
- `deepseek-v4-flash` — 快速版本

**API 端点**：`https://api.deepseek.com/v1`

---

## 项目结构

```
Mimo-TUI/
├── Cargo.toml                    # Workspace 根配置
├── Cargo.lock
├── README.md                     # 本文件
├── MIMO_TUI_ARCHITECTURE.md      # 架构设计蓝图
│
├── packages/
│   ├── core/                     # 核心引擎 (mimo-core)
│   │   └── src/
│   │       ├── engine/           #   智能体引擎、对话轮次、上下文管理
│   │       ├── memory/           #   记忆管理器、档案检索 (BM25)
│   │       ├── session/          #   会话管理、检查点
│   │       ├── subagent/         #   子智能体管理、消息路由
│   │       ├── tools/            #   工具注册表、执行器、类型定义
│   │       └── lib.rs
│   │
│   ├── providers/                # LLM Provider 实现 (mimo-providers)
│   │   └── src/
│   │       ├── provider.rs       #   LLMProvider trait 定义
│   │       ├── mimo.rs           #   MiMo Provider
│   │       ├── deepseek.rs       #   DeepSeek Provider
│   │       └── lib.rs
│   │
│   └── apps/
│       ├── cli/                  # CLI 应用 (mimo-cli)
│       │   └── src/main.rs
│       │
│       └── tui/                  # TUI 应用 (mimo-tui)
│           └── src/
│               ├── app.rs        #   主 App 控制器
│               ├── main.rs       #   入口点
│               ├── components/   #   UI 组件
│               │   ├── mod.rs
│               │   ├── approval.rs  # 审批弹窗
│               │   ├── command.rs   # 命令解析
│               │   ├── header.rs    # 标题栏
│               │   ├── input.rs     # 输入组件
│               │   └── messages.rs  # 消息渲染
│               └── views/        #   视图
│                   ├── mod.rs
│                   ├── chat.rs      # 对话视图
│                   ├── help.rs      # 帮助视图
│                   ├── agents.rs    # SubAgent 视图
│                   └── history.rs   # 会话历史视图
│
├── crates/
│   ├── types/                    # 共享类型定义
│   ├── config/                   # 配置管理 (TOML)
│   ├── hooks/                    # 生命周期钩子系统
│   └── execpolicy/               # 执行策略引擎（安全检查）
│
└── docs/
    ├── MEMORY.md                 # 记忆系统文档
    ├── SUBAGENT_PROMPTS.md       # SubAgent 提示词模板
    └── TUI_DEVELOPMENT_PROMPTS.md
```

---

## 开发指南

### 常用命令

```bash
# 编译检查
cargo check

# 编译
cargo build

# 发布编译（优化）
cargo build --release

# 运行测试
cargo test --workspace --all-features

# 代码格式化
cargo fmt --all

# Clippy 检查
cargo clippy --workspace --all-targets --all-features

# 运行特定二进制
cargo run --bin mimo-tui    # TUI 版本
cargo run --bin mimo         # CLI 版本
```

### 技术栈

| 类别 | 技术 | 说明 |
|------|------|------|
| 语言 | Rust 2024 Edition | 系统编程语言 |
| 异步运行时 | Tokio | 异步 I/O 和任务调度 |
| TUI 框架 | ratatui 0.29 | 终端用户界面 |
| 终端控制 | crossterm 0.28 | 跨平台终端操作 |
| HTTP 客户端 | reqwest 0.12 | HTTP 请求（rustls-tls） |
| 序列化 | serde / serde_json | JSON 序列化 |
| 错误处理 | anyhow / thiserror | 结构化错误处理 |
| 日期时间 | chrono 0.4 | 时间处理 |
| UUID | uuid 1.0 | 唯一标识符生成 |

### 架构设计

详细架构文档请参阅 [MIMO_TUI_ARCHITECTURE.md](MIMO_TUI_ARCHITECTURE.md)。

核心分层：

```
User Interfaces  →  Presentation Layer  →  Application Layer
                                        →  Engine Layer
                                        →  LLM Provider Layer
                                        →  External Services
```

### 添加新工具

1. 在 [registry.rs](packages/core/src/tools/registry.rs) 中注册工具名称
2. 在 [runner.rs](packages/core/src/tools/runner.rs) 中实现工具的执行逻辑
3. 在 [app.rs](packages/apps/tui/src/app.rs) 的 `tool_to_definition` 中添加 JSON Schema 定义

### 添加新 Provider

1. 创建新的 Provider 结构体，实现 `LLMProvider` trait
2. 在 [provider.rs](packages/providers/src/provider.rs) 中定义数据模型
3. 在 [config.rs](crates/config/src/config.rs) 中添加 Provider 枚举变体

---

## License

MIT — 详见 [LICENSE](LICENSE) 文件。
