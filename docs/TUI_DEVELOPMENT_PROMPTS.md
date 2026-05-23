# MiMo-TUI 完整实现

***

## 📋 项目总览（必读）

### 项目背景

MiMo-TUI 是一个基于 Rust + ratatui 的终端智能编程助手，后端以小米 MiMo V2.5 Pro 为核心模型。项目采用 workspace 结构，TUI 是其中的一个前端应用。

### 项目结构

```
Mimo-TUI/
├── packages/
│   ├── core/              # 核心引擎
│   │   └── src/
│   │       ├── engine/     # Agent 引擎、TurnLoop、ContextManager
│   │       ├── session/    # 会话管理、检查点
│   │       ├── tools/      # 工具注册、执行器
│   │       ├── subagent/   # 子智能体管理
│   │       └── memory/     # 记忆系统（MemoryManager、RecallArchive）
│   ├── providers/          # LLM Provider（MiMo、DeepSeek 等）
│   └── apps/
│       ├── cli/            # CLI 应用（已完整可用）
│       └── tui/            # TUI 应用（需要实现）← 你的任务
├── crates/                 # 共享 crate
│   ├── types/              # 公共类型
│   ├── config/             # 配置管理
│   ├── hooks/              # 生命周期钩子
│   └── execpolicy/         # 审批策略
└── Cargo.toml              # workspace 定义
```

### 技术栈

| 依赖             | 版本         | 用途              |
| -------------- | ---------- | --------------- |
| ratatui        | 0.29       | TUI 渲染框架        |
| crossterm      | 0.28       | 终端控制（输入、光标、颜色）  |
| tokio          | 1.x (full) | 异步运行时           |
| anyhow         | 1.x        | 错误处理            |
| tracing        | 0.1        | 日志              |
| unicode-width  | 0.2        | Unicode 字符串宽度计算 |
| mimo-core      | path       | 核心引擎            |
| mimo-providers | path       | LLM Provider    |

### 现有代码状态（你需要改造的骨架代码）

**`packages/apps/tui/src/main.rs`** - 入口，已正确调用 `run()`

**`packages/apps/tui/src/app.rs`** - App 结构极其简单，只有一个 `should_quit` 标志和 `title`。事件循环没有处理任何键盘输入，也没有任何交互。需要你完整重写。

**`packages/apps/tui/src/components/input.rs`** - 简单的输入组件，有基本的字符插入、退格、光标移动。但不支持多行、不支持渲染光标位置、没有样式。

**`packages/apps/tui/src/components/messages.rs`** - 消息组件，`Vec<String>` 存储消息，用 `List` 渲染。没有角色区分、没有 Markdown 渲染、没有流式更新支持。

**`packages/apps/tui/src/views/chat.rs`** - 简单的分屏布局（消息区 + 输入区）。没有事件处理、没有与引擎集成。

**`packages/apps/tui/src/views/help.rs`** - 帮助视图，硬编码文本。

### 核心引擎 API（你需要调用的）

从 `mimo_core` 和 `mimo_providers` 暴露的关键类型：

```rust
// 会话
use mimo_core::{SessionManager, Session, session::MessageRole};

// 工具
use mimo_core::{ToolRegistry, ToolCall, ToolResult, ToolRunner};

// 记忆
use mimo_core::{MemoryManager, RecallArchive};

// 子智能体
use mimo_core::{SubAgentManager, SubAgent, SubAgentRole, SubAgentStatus};

// 引擎
use mimo_core::{AgentEngine, ContextManager};

// LLM Provider
use mimo_providers::{MimoProvider, LLMProvider};
use mimo_providers::provider::{Message as ProviderMessage, MessageRole as ProviderMessageRole, ChatOptions, ToolDefinition};
```

### 参考实现

CLI 应用 (`packages/apps/cli/src/main.rs`) 已经完整实现了：

- API 认证和连接
- 消息处理循环
- 工具调用执行
- 会话持久化
- 记忆系统（`/memory` 命令、`#` 快捷输入）
- 帮助命令

你可以参考 CLI 的实现逻辑，将其迁移到 TUI 的事件驱动模型中。

### 设计目标

```
┌─────────────────────────────────────────────────────────────────┐
│  MiMo-TUI                                    [Model: MiMo v2.5] │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                     Messages Area                            │ │
│  │  ┌────────────────────────────────────────────────────────┐ │ │
│  │  │ [User]                                                │ │ │
│  │  │ 请帮我分析这个模块的架构                                │ │ │
│  │  └────────────────────────────────────────────────────────┘ │ │
│  │  ┌────────────────────────────────────────────────────────┐ │ │
│  │  │ [Assistant]  ◐ 正在思考...                              │ │ │
│  │  │                                                      │ │ │
│  │  │ ┌─ Thinking ────────────────────────────────────────┐ │ │ │
│  │  │ │ 我需要先了解这个模块的文件结构...                    │ │ │
│  │  │ └────────────────────────────────────────────────────┘ │ │ │
│  │  │                                                      │ │ │
│  │  │ 这个模块采用分层架构：                                 │ │ │
│  │  │ • 表现层（Presentation）                              │ │ │
│  │  └────────────────────────────────────────────────────────┘ │ │
│  │  ┌────────────────────────────────────────────────────────┐ │ │
│  │  │ [Tool Call] exec_shell                                 │ │ │
│  │  │ $ find src/ -name "*.rs" | head -20                   │ │ │
│  │  │ ✓ Completed in 0.12s                                   │ │ │
│  │  └────────────────────────────────────────────────────────┘ │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────┐ ┌───────────┐ │
│  │ > 请输入消息...                    [Ctrl+Enter] │ │ [Send]   │ │
│  └─────────────────────────────────────────────┘ └───────────┘ │
├─────────────────────────────────────────────────────────────────┤
│  [Tab] Chat  [Tab] SubAgents  [Tab] History  [?] Help  [/cmd]  │
└─────────────────────────────────────────────────────────────────┘
```

***

## 任务 1: 事件循环与终端生命周期

### 当前代码

```rust
// app.rs 中的 run_app
fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| {
            let area = f.area();
            let block = ratatui::widgets::Block::default()
                .title(app.title())
                .borders(ratatui::widgets::Borders::ALL);
            f.render_widget(block, area);
        })?;

        if app.should_quit() {
            break;
        }
    }
    Ok(())
}
```

**问题**：没有键盘事件处理，无法退出（只能 Ctrl+C 强杀），没有事件轮询。

### 要求

1. **实现事件轮询**：使用 `crossterm::event::poll()` 和 `event::read()` 处理键盘事件
2. **支持退出机制**：
   - `Ctrl+C` → 退出
   - `Esc` → 取消当前操作 / 退出命令模式
   1. **支持全屏重绘**：`Ctrl+L` 或 `clear`清屏并重绘
3. **终端生命周期管理**：
   - 启动时启用 raw mode、进入备用屏幕、启用鼠标捕获
   - 退出时清理（已有，保持不变）
   - 确保 panic 时也能正确清理（使用 `panic::catch_unwind` 或 RAII guard）
4. **App 结构扩展**：添加 `focused` 字段（输入区 / 消息区）、`mode` 字段（Normal / Insert / Command）

### 期望的 App 结构

```rust
pub struct App {
    pub should_quit: bool,
    pub title: String,
    pub mode: AppMode,       // Normal, Insert, Command
    pub status_message: Option<String>,
    pub status_type: StatusType, // Info, Warning, Error
}

pub enum AppMode {
    Normal,      // 正常浏览模式
    Insert,      // 输入模式
    Command,     // 命令模式（输入 / 开头）
}
```

### 期望的行为

```
用户按键        行为
─────────       ────
q / Ctrl+C      设置 should_quit = true，退出循环
i               进入 Insert 模式，聚焦输入区
: 或 /          进入 Command 模式
Ctrl+L          清屏并重绘
Esc             返回 Normal 模式
Tab             切换视图（Chat → SubAgents → History）
```

### 交付物

- 修改 `app.rs`：扩展 App 结构，实现完整事件循环
- 确保编译通过且可以正常启动和退出

***

## 任务 2: 分屏布局系统

### 当前代码

`views/chat.rs` 有一个简单的垂直两分区布局（消息区 + 输入区）。

### 要求

1. **实现完整的分屏布局**：
   ```
   ┌──────────────────────────────────────────┐
   │  Header: MiMo-TUI          [Model] [Mem] │
   ├──────────────────────────────────────────┤
   │                                          │
   │           Messages Area                   │
   │                                          │
   ├──────────────────────────────────────────┤
   │  Input Area                               │
   ├──────────────────────────────────────────┤
   │  Footer: [Chat] [Agents] [History]  [?]  │
   └──────────────────────────────────────────┘
   ```
2. **各区域约束**：
   - Header: 固定 1 行
   - Messages: `Min(1)` - 占据剩余空间
   - Input: 固定 3 行（支持多行输入可扩展）
   - Footer: 固定 1 行
3. **布局响应式**：终端大小变化时自动重排
4. **焦点高亮**：当前聚焦的区域用不同边框颜色/样式标识
5. **使用** **`ratatui::layout`** **的** **`Layout`** **+** **`Constraint`** **实现**

### 交付物

- 修改 `views/chat.rs`：实现完整布局
- 创建 `views/mod.rs` 中管理视图切换逻辑

***

## 任务 3: 输入组件（完整实现）

### 当前代码

`components/input.rs` 有基本的字符操作，但问题：

- 使用 `self.value.len()` 计算光标位置（对 Unicode 不正确）
- 没有多行支持
- 渲染时不显示光标
- 没有样式

### 要求

1. **Unicode 安全**：使用 `unicode-width` crate 和字符迭代，而非字节索引
2. **光标渲染**：使用 `SetCursor` 或 `Span` 样式在正确位置显示光标
3. **多行支持**：支持 `Enter` 换行（除非在发送消息时）
4. **快捷键**：
   - `Enter`（带 Ctrl）：发送消息
   - `Esc`：取消输入
   - `Ctrl+A` / `Home`：光标到行首
   - `Ctrl+E` / `End`：光标到行尾
   - `Ctrl+W`：删除前一个单词
   - `Ctrl+U`：删除到行首
   - `Ctrl+K`：删除到行尾
   - `↑/↓`：浏览历史输入
5. **输入历史记录**：保存最近的 N 条输入（默认 50），支持上下箭头浏览
6. **自动换行**：超过输入框宽度时自动换行

### 期望的 API

```rust
pub struct InputComponent {
    // ...
}

impl InputComponent {
    pub fn new() -> Self;
    pub fn handle_key(&mut self, key: KeyEvent) -> InputAction;
    pub fn render(&self, f: &mut Frame, area: Rect, focused: bool);
    pub fn value(&self) -> &str;
    pub fn clear(&mut self);
    pub fn set_placeholder(&mut self, text: String);
}

pub enum InputAction {
    None,
    Submit(String),    // 用户发送了消息
    Cancel,            // 用户取消输入
    NavigateHistory(Direction), // 浏览历史
}
```

### 交付物

- 重写 `components/input.rs`：完整的输入组件
- 确保 Unicode 正确处理

***

## 任务 4: 消息渲染组件

### 当前代码

`components/messages.rs` 只是 `Vec<String>` 的 `List` 渲染。

### 要求

1. **角色区分**：消息结构应包含角色信息：
   ```rust
   pub struct ChatMessage {
       pub id: String,
       pub role: MessageRole,  // User, Assistant, System, Tool
       pub content: String,
       pub timestamp: DateTime<Utc>,
       pub thinking: Option<String>,  // 思考内容
       pub tool_calls: Option<Vec<ToolCallInfo>>,  // 工具调用信息
   }
   ```
2. **样式区分**：
   - User 消息：右侧对齐，特定背景色
   - Assistant 消息：左侧对齐，不同背景色
   - Tool 调用：带边框的卡片样式，显示工具名、参数、执行状态
   - System 消息：灰色斜体
3. **Markdown 渲染**：
   - 至少支持：粗体、斜体、代码块、行内代码、列表、链接
   - 代码块需要语法高亮（如果可行，用简单的关键词高亮；否则用不同颜色块）
   - 行内代码用不同颜色背景
4. **思考过程展示**：
   - 折叠/展开切换
   - 用特殊样式（灰色、缩进）区分思考内容和最终回答
   - 支持流式更新（打字机效果）
5. **自动滚动**：新消息到达时自动滚到底部
6. **长消息处理**：超过屏幕时支持上下滚动

### 交付物

- 重写 `components/messages.rs`：完整的消息组件
- 支持角色样式、Markdown 子集、思考展示

***

## 任务 5: Chat 视图集成 MiMo 引擎

### 要求

这是最核心的任务 —— 让 TUI 真正能与 MiMo 对话。

1. **初始化流程**：
   ```
   1. 读取 MIMO_API_KEY 环境变量
   2. 创建 MimoProvider
   3. 执行 health_check
   4. 创建 SessionManager、ToolRegistry、ToolRunner、MemoryManager
   5. 创建新会话
   6. 显示状态（API 连接成功/失败、记忆开启/关闭）
   ```
2. **消息发送流程**（用户按 Ctrl+Enter 后）：
   ```
   1. 获取输入内容
   2. 添加到会话消息
   3. 构建 ProviderMessage 列表（注入记忆块到第一条消息）
   4. 调用 provider.chat_completions()
   5. 流式渲染响应（逐字符/逐词更新）
   6. 处理工具调用：
      a. 解析 tool_calls
      b. 执行工具（通过 ToolRunner）
      c. 将结果作为 Tool 消息追加
      d. 继续调用 LLM（直到没有更多 tool_calls）
   7. 保存会话检查点
   ```
3. **异步处理**：
   - 使用 `tokio` 异步运行时
   - LLM 调用在异步任务中执行
   - 通过 channel（`tokio::sync::mpsc`）将更新发送到 UI 线程
   - UI 在等待响应时显示 "正在思考..." 或进度指示
4. **错误处理**：
   - API 错误 → 在状态栏显示错误
   - 网络超时 → 显示重试提示
   - 工具执行失败 → 显示错误并追加到消息历史
5. **参考 CLI 实现**：
   - `process_message` 函数的逻辑（`packages/apps/cli/src/main.rs` 第 287 行开始）
   - `build_tools_from_registry` 工具注册
   - 记忆块注入逻辑
   - 工具调用循环

### 交付物

- 修改 `views/chat.rs`：集成完整对话流程
- 支持异步消息流、工具调用、错误处理

***

## 任务 6: 记忆系统集成

### 要求

将记忆系统完整集成到 TUI 中：

1. **启动时**：
   ```rust
   let memory_enabled = std::env::var("MIMO_MEMORY").is_ok();
   let memory_manager = MemoryManager::new(&storage_path, memory_enabled);
   ```
2. **侧边栏/状态栏显示记忆状态**：
   ```
   Header 右侧: [🧠 Memory: ON | 3 entries]  或  [🧠 Memory: OFF]
   ```
3. **消息注入**：每次发送消息前，注入记忆块：
   ```rust
   let memory_block = memory_manager.compose_block();
   if let Some(block) = memory_block {
       messages[0].content = format!("{}\n\n{}", block, messages[0].content);
   }
   ```
4. **`/memory`** **命令**：
   - `/memory` 或 `/memory show` → 弹出窗口显示当前记忆
   - `/memory add <内容>` → 添加记忆条目
   - `/memory clear` → 清空记忆（需要确认）
   - `#<内容>` 快捷输入 → 直接添加到记忆
5. **记忆浏览弹窗**：
   ```
   ┌── Memory (3 entries) ────────────────────────────┐
   │ - (2026-05-16 10:30 UTC) prefer pytest over unit  │
   │ - (2026-05-16 10:31 UTC) 4-space indentation      │
   │ - (2026-05-16 10:32 UTC) use rust for system tools│
   │                                                    │
   │ [q] Close  [c] Clear  [a] Add entry               │
   └───────────────────────────────────────────────────┘
   ```

### 交付物

- 在 App 结构中添加 MemoryManager
- 在 Header 显示记忆状态
- 实现 `/memory` 命令处理
- 支持 `#` 快捷记忆输入

***

## 任务 7: 命令处理系统

### 要求

实现 TUI 的命令系统，支持以下命令：

| 命令                | 功能                        |
| ----------------- | ------------------------- |
| `/help`           | 显示帮助弹窗（已有 `HelpView`，需完善） |
| `/memory`         | 查看/管理记忆                   |
| `/memory add <n>` | 添加记忆                      |
| `/memory clear`   | 清空记忆                      |
| `/tools`          | 显示可用工具列表                  |
| `/sessions`       | 显示会话列表                    |
| `/session <id>`   | 切换到指定会话                   |
| `/clear`          | 清空当前对话消息                  |
| `/compact`        | 手动触发上下文压缩                 |
| `/model`          | 显示当前模型                    |
| `/quit` 或 `/exit` | 退出应用                      |

### 实现方式

```rust
pub enum Command {
    Help,
    Memory { subcmd: MemorySubCommand },
    Tools,
    Sessions,
    SwitchSession(String),
    Clear,
    Compact,
    Model,
    Quit,
    Unknown(String),
}

pub fn parse_command(input: &str) -> Command {
    let parts: Vec<&str> = input.split_whitespace().collect();
    match parts.first() {
        Some(&"/help") => Command::Help,
        Some(&"/memory") => parse_memory_command(&parts),
        // ...
        _ => Command::Unknown(input.to_string()),
    }
}
```

### 命令执行

- 命令执行结果通过 `status_message` 显示在状态栏
- 部分命令弹出浮窗（帮助、工具列表、记忆浏览）

### 交付物

- 创建 `components/command.rs`：命令解析和执行
- 在事件循环中处理 Command 模式输入

***

## 任务 8: 流式响应与打字机效果

### 要求

MiMo 的 API 支持流式响应（Server-Sent Events），需要在 TUI 中实时渲染。

1. **流式处理**：
   ```
   收到 response_delta → 追加到当前消息缓冲区 → 重绘屏幕
   ```
2. **打字机效果**：
   - 不要一次性渲染完整响应
   - 每收到一个 chunk 就更新显示
   - 给用户"正在输入"的感觉
3. **流式状态指示**：
   - 思考中：显示 `◐ 正在思考...`
   - 工具调用中：显示 `⚙ 执行工具: xxx`
   - 流式输出中：显示光标闪烁或 `...`
4. **中断支持**：
   - `Ctrl+C` 在流式响应中 → 中断当前请求
   - 显示 "已中断" 标记

### 实现方式

```rust
// 使用 channel 传递流式更新
enum StreamEvent {
    ThinkingDelta(String),    // 思考内容增量
    ContentDelta(String),     // 响应内容增量
    ToolCallStart { name, args },
    ToolCallResult { name, result },
    Complete,
    Error(String),
}

// UI 线程接收并应用
loop {
    select! {
        event = rx.recv() => {
            match event {
                Some(StreamEvent::ContentDelta(text)) => {
                    current_message.content.push_str(&text);
                    // 重绘
                }
                // ...
            }
        }
        _ = event::poll(timeout) => {
            // 处理键盘事件
        }
    }
}
```

### 交付物

- 实现流式响应的 channel 通信
- 实现打字机效果的 UI 更新
- 支持中断流式响应

***

## 任务 9: 子智能体监控视图

### 要求

实现 SubAgents 视图，用于监控子智能体的状态。

1. **视图布局**：
   ```
   ┌── SubAgents (3 active) ──────────────────────────┐
   │                                                    │
   │  ID              Role          Status     Progress │
   │  ─────────────   ──────────    ───────    ───────  │
   │  abc-123         Explore       🔄 Running  ███░░ 60%│
   │  def-456         Implementer   ✅ Done     █████ 100%│
   │  ghi-789         Plan          ⏳ Pending  ░░░░░  0%│
   │                                                    │
   │  [r] Refresh  [c] Cancel  [d] Details  [q] Back   │
   └───────────────────────────────────────────────────┘
   ```
2. **功能**：
   - 实时刷新子智能体状态
   - 查看单个子智能体的详细信息（任务描述、输出日志）
   - 取消正在运行的子智能体
   - 查看子智能体结果

### 交付物

- 创建 `views/agents.rs`：子智能体监控视图
- 支持刷新、查看、取消操作

***

## 任务 10: 会话历史视图

### 要求

实现 History 视图，用于查看和切换会话。

1. **视图布局**：
   ```
   ┌── Sessions ───────────────────────────────────────┐
   │                                                    │
   │  Created         Title                  Messages   │
   │  ─────────────   ──────────────────────   ───────  │
   │  10:30           重构用户认证模块            12     │
   │  09:45           分析数据库连接池            8     │
   │  昨天            编写单元测试               24     │
   │                                                    │
   │  [Enter] Open  [d] Delete  [q] Back               │
   └───────────────────────────────────────────────────┘
   ```
2. **功能**：
   - 列出所有历史会话
   - 使用第一消息作为会话标题预览
   - 切换到之前的会话
   - 删除不需要的会话

### 交付物

- 创建 `views/history.rs`：会话历史视图
- 支持列表、切换、删除操作

***

## 任务 11: 审批交互界面

### 要求

当工具调用需要用户审批时，弹出审批对话框。

```
┌── Tool Approval ─────────────────────────────────────┐
│                                                       │
│  Tool: shell                                          │
│  Arguments:                                           │
│    command: "rm -rf /tmp/old-build"                   │
│                                                       │
│  Risk: 🔴 High (destructive command)                  │
│                                                       │
│  [a] Approve  [r] Reject  [A] Approve All  [q] Cancel│
│                                                       │
└───────────────────────────────────────────────────────┘
```

1. **功能**：
   - 显示工具名称和参数
   - 风险评估（低/中/高）
   - 审批选项：批准、拒绝、全部批准、取消
   - 超时自动拒绝（可配置）

### 交付物

- 创建 `components/approval.rs`：审批对话框
- 集成到工具调用流程中

***

## 任务 12: 标题栏与状态栏

### 要求

1. **标题栏（Header）**：
   ```
   MiMo-TUI                                    [MiMo v2.5] [🧠 ON]
   ```
   - 左侧：应用名称
   - 右侧：当前模型、记忆状态
2. **状态栏（Footer）**：
   ```
   [Chat] [Agents(2)] [History]           Normal  |  q:Quit  ?:Help
   ```
   - 左侧：视图切换标签
   - 中间：当前模式
   - 右侧：快捷键提示
3. **动态状态消息**（覆盖在 Footer 上方）：
   ```
   ✓ Memory added: "prefer pytest"
   ✗ API Error: rate limit exceeded
   ⏳ Connecting to MiMo API...
   ```
   - 3 秒后自动消失
   - 支持 Info、Success、Warning、Error 类型

### 交付物

- 创建 `components/header.rs`：标题栏组件
- 创建 `components/footer.rs`：状态栏组件
- 创建 `components/status_bar.rs`：动态状态消息

***

## 任务 13: 键盘导航与滚动

### 要求

1. **消息区滚动**：
   - `j` / `↓`：向下滚动
   - `k` / `↑`：向上滚动
   - `g`：跳到顶部
   - `G`：跳到底部
   - `Ctrl+D`：向下半页
   - `Ctrl+U`：向上半页
2. **列表导航**（会话列表、工具列表等）：
   - `j` / `k`：上下移动
   - `Enter`：选择
   - `Esc`：返回
3. **鼠标支持**（可选）：
   - 滚轮滚动消息区
   - 点击切换视图标签

### 交付物

- 在 App 中添加滚动状态
- 在各个视图中处理导航键

***

## 任务 14: 配置管理与环境变量

### 要求

1. **环境变量支持**：
   ```bash
   MIMO_API_KEY=xxx          # API 密钥
   MIMO_MEMORY=on            # 启用记忆
   MIMO_MODEL=mimo-v2.5-pro  # 默认模型
   MIMO_DEBUG=on             # 调试模式（显示更多日志）
   ```
2. **配置文件**（可选，未来扩展）：
   ```toml
   # ~/.mimo/config.toml
   [general]
   model = "mimo-v2.5-pro"
   temperature = 0.7

   [memory]
   enabled = true

   [ui]
   theme = "dark"
   show_thinking = true
   ```

### 交付物

- 创建 `config.rs`（或利用 `crates/config`）
- 在启动时加载配置

***

## 任务 15: 错误处理与用户反馈

### 要求

1. **API 错误**：
   - 401：API 密钥无效 → 提示用户检查 `MIMO_API_KEY`
   - 429：速率限制 → 提示等待后重试
   - 500：服务器错误 → 提示稍后重试
2. **网络错误**：
   - 超时 → 显示超时提示和重试选项
   - 无连接 → 显示网络错误提示
3. **工具执行错误**：
   - 文件不存在 → 显示具体文件路径
   - 命令失败 → 显示 stdout/stderr 输出
   - 权限不足 → 提示用户
4. **用户反馈**：
   - 所有操作都有视觉反馈（状态消息、Toast 提示）
   - 长时间操作显示进度指示
   - 错误提供可操作的建议

### 交付物

- 统一的错误展示系统
- 在状态栏和消息区显示错误

***

## 📐 开发顺序建议

```
Phase 1 (核心可用):
  1 → 事件循环与终端生命周期
  2 → 分屏布局系统
  3 → 输入组件
  7 → 命令处理系统
  5 → Chat 视图集成 MiMo 引擎（基础对话）

Phase 2 (完整功能):
  4 → 消息渲染组件（Markdown、角色样式）
  8 → 流式响应与打字机效果
  6 → 记忆系统集成
  12 → 标题栏与状态栏
  13 → 键盘导航与滚动

Phase 3 (高级特性):
  9 → 子智能体监控视图
  10 → 会话历史视图
  11 → 审批交互界面
  14 → 配置管理
  15 → 错误处理与用户反馈
```

***

## ⚠️ 开发注意事项

1. **始终使用** **`cargo check`** **验证编译**
2. **遵循 Rust 惯例**：命名、错误处理（anyhow/Result）、模块化
3. **模仿现有代码风格**：参考 CLI 的实现模式
4. **Unicode 安全**：所有字符串操作使用字符迭代而非字节索引
5. **异步兼容**：UI 渲染在同步线程，LLM 调用在异步任务，通过 channel 通信
6. **终端清理**：panic 时也要确保 raw mode 被禁用
7. **不要提交敏感信息**：API 密钥通过环境变量读取

***

*提示词版本：v1.0 | 创建日期：2026-05-16*
