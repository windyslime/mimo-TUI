# MiMo-TUI 记忆系统重构 — AI Agent 实现提示词

> 共 3 个 Agent，分别负责不同模块。每个 Agent 独立工作，遵循统一的接口契约。
> 所有 Agent 必须在实现前**先通读接口契约章节**，确保输出与其他 Agent 兼容。

---

## 📋 接口契约（所有 Agent 必须遵守）

### 公共类型定义

```rust
// 记忆目标类型
pub enum MemoryTarget {
    Memory,  // Agent 个人笔记 (MEMORY.md)
    User,    // 用户画像 (USER.md)
}

// 记忆操作的 JSON 结果
// 所有 memory 工具返回 JSON 字符串，格式：
// {"ok": true, "target": "memory", "usage_pct": 67, "usage_chars": 1474, "limit_chars": 2200}
// 或
// {"ok": false, "error": "Memory full: 2180/2200 chars used. Need 120 more chars."}
```

### MemoryManager 的公开 API（Agent 1 对外暴露）

```rust
impl MemoryManager {
    // 构造
    pub fn new(base_dir: &Path) -> Self;

    // 冻结快照（会话开始时调用一次）
    pub fn take_snapshot(&mut self);
    pub fn get_system_prompt_block(&self) -> Option<String>;  // 用于注入 system prompt

    // 工具操作（会立即持久化到磁盘，但不影响快照）
    pub fn add(&self, target: MemoryTarget, content: &str) -> MemoryOpResult;
    pub fn replace(&self, target: MemoryTarget, old_text: &str, new_content: &str) -> MemoryOpResult;
    pub fn remove(&self, target: MemoryTarget, old_text: &str) -> MemoryOpResult;

    // 查询
    pub fn memory_usage(&self, target: MemoryTarget) -> MemoryUsage;  // {pct, chars, limit_chars}
    pub fn list_entries(&self, target: MemoryTarget) -> Vec<String>;  // 实时条目列表
}

// MemoryOpResult: 操作结果
pub struct MemoryOpResult {
    pub ok: bool,
    pub message: String,         // 人类可读的描述
    pub target: MemoryTarget,
    pub usage_pct: u32,          // 0-100
    pub usage_chars: usize,
    pub limit_chars: usize,
}
```

### ToolHandler 枚举（Agent 2 定义，Agent 1/3 使用）

```rust
pub enum ToolHandler {
    // ... 保留原有 ...
    Remember,       // 行为变更: 支持 target 参数
    MemoryReplace,  // 新增
    MemoryRemove,   // 新增
    RecallArchive,
    // 移除: MemoryOrganize
}
```

### ToolRunner 构造函数变更（Agent 2 定义）

```rust
impl ToolRunner {
    // 不再接受 Option<MemoryManager>，改为必定持有
    pub fn new(memory_manager: MemoryManager, recall_archive: Option<RecallArchive>) -> Self;
}
```

### System Prompt 注入方式（Agent 3 实现）

```rust
// 在构建消息列表时，将会话开始时拍的快照作为 system 消息插入到最前面
// 格式:
// system: "<memory_prompt>\n\n你是一个编程助手 MiMo...\n\n═══════════════════MEMORY═══════════════════\n- (2026-01-15) 条目1\n- (2026-01-16) 条目2\n════════════════════USER PROFILE════════════════════\n- 条目A\n"

// 该 system 消息在整个会话期间不变（冻结快照）
// 用户的第一条消息不再包含记忆块前缀
```

---

## Agent 1: MemoryManager 双文件重构

### 你需要修改的文件
- `packages/core/src/memory/memory.rs` — 完全重写
- `packages/core/src/memory/mod.rs` — 更新 exports

### 你需要阅读的参考文件（只读）
- `packages/core/src/memory/recall.rs` — 不需要改，了解路径约定

### 任务描述

将现有的单文件 `~/.mimo/memory.md` 记忆系统重构为双文件 + 冻结快照模式，对标 Hermes Agent 的记忆设计。

#### 1.1 存储路径变更

| 旧 | 新 |
|---|---|
| `~/.mimo/memory.md` | `~/.mimo/memories/MEMORY.md` |
| — | `~/.mimo/memories/USER.md` |

#### 1.2 移除旧的常量
删除以下常量：
- `DEFAULT_MEMORY_PATH`
- `MAX_MEMORY_SIZE`（用字符限制替代）

#### 1.3 新的字符限制常量

```rust
pub const MEMORY_MD_CHAR_LIMIT: usize = 3000;   // MEMORY.md 字符上限
pub const USER_MD_CHAR_LIMIT: usize = 1500;      // USER.md 字符上限
pub const ENTRY_DELIMITER: &str = "§";           // 条目分隔符（Hermes 风格）
```

#### 1.4 MemoryManager 结构体

```rust
pub struct MemoryManager {
    memory_path: PathBuf,      // ~/.mimo/memories/MEMORY.md
    user_path: PathBuf,        // ~/.mimo/memories/USER.md
    // 冻结快照（会话开始时拍，整个会话不变）
    snapshot_memory: Option<String>,
    snapshot_user: Option<String>,
}
```

不再有 `enabled` 字段 —— 记忆始终启用（简化设计）。

#### 1.5 核心方法实现

##### `new(base_dir: &Path) -> Self`
- `memory_path` = `base_dir.join("memories").join("MEMORY.md")`
- `user_path` = `base_dir.join("memories").join("USER.md")`
- 快照初始化为 None
- 确保 `memories/` 目录存在

##### `take_snapshot(&mut self)`
- 读取 `memory_path` 内容，存入 `snapshot_memory`
- 读取 `user_path` 内容，存入 `snapshot_user`
- 如果文件不存在，快照为 `None`

##### `get_system_prompt_block(&self) -> Option<String>`
- 使用快照内容（不是磁盘实时内容）
- 格式如下（Hermes 风格）：

```
══════════════════════════════════════════════MEMORY (your personal notes) [67% — 1,474/2,200 chars]══════════════════════════════════════════════
条目1§条目2§条目3
═══════════════════════════════════════════USER PROFILE [53% — 732/1,375 chars]═══════════════════════════════════════════
用户喜欢的编码风格§偏好 Python
```

- 条目用 `§` 分隔
- 如果两个快照都为空，返回 `None`
- 标题行使用 `═` 字符
- 使用率百分比和字符计数必须准确

##### `add(&self, target: MemoryTarget, content: &str) -> MemoryOpResult`
- 根据 `target` 选择 `MEMORY.md` 或 `USER.md`
- 追加新条目（使用 `§` 分隔符）
- 追加前检查字符上限：如果当前字符数 + 新内容长度 + 分隔符 > 上限，返回 `ok: false` 和错误信息
- 追加后返回包含使用率的 `MemoryOpResult`
- 使用原子写入（临时文件 + `fs::rename`，参考 Hermes 的 `_write_file`）
- 文件不存在时创建

##### `replace(&self, target: MemoryTarget, old_text: &str, new_content: &str) -> MemoryOpResult`
- 读取所有条目，用 `§` 分割
- 使用**子字符串匹配**找到包含 `old_text` 的条目
- 如果匹配到多个条目，返回错误（要求更精确的匹配）
- 如果匹配到 0 个，返回错误
- 替换该条目为 `new_content`
- 原子写入，返回操作结果

##### `remove(&self, target: MemoryTarget, old_text: &str) -> MemoryOpResult`
- 与 replace 类似的子字符串匹配
- 删除匹配的条目
- 原子写入

##### `memory_usage(&self, target: MemoryTarget) -> MemoryUsage`
- 返回当前磁盘文件的实际使用情况（不是快照）
- 读取文件内容，计算字符数

##### `list_entries(&self, target: MemoryTarget) -> Vec<String>`
- 读取磁盘文件（实时内容，不是快照）
- 用 `§` 分割返回条目列表
- 过滤空字符串

#### 1.6 需要移除的方法
- `from_path()` — 简化
- `is_enabled()`, `path()`, `load()`, `clear()`, `entries()`, `replace_entries()`, `compose_block()`, `entry_count()`, `size_bytes()`
- `parse_entry()` — 不再需要时间戳解析
- `MemoryEntry` 结构体 — 不再需要

#### 1.7 mod.rs 更新
```rust
pub mod memory;
pub mod recall;

pub use memory::{MemoryManager, MemoryTarget, MemoryOpResult, MemoryUsage};
pub use recall::{RecallArchive, RecallHit};
```

#### 1.8 需要新增的类型

```rust
// 在 memory.rs 中定义
pub enum MemoryTarget { Memory, User }

pub struct MemoryUsage {
    pub pct: u32,
    pub chars: usize,
    pub limit_chars: usize,
}

pub struct MemoryOpResult {
    pub ok: bool,
    pub message: String,
    pub target: MemoryTarget,
    pub usage_pct: u32,
    pub usage_chars: usize,
    pub limit_chars: usize,
}
```

#### 1.9 原子写入实现
参考如下伪代码：
```rust
fn atomic_write(path: &Path, content: &str) -> std::io::Result<()> {
    let dir = path.parent().unwrap();
    let (fd, tmp_path) = tempfile::mkstemp_in(dir)?;  // 或手动实现
    // 写入临时文件
    // fsync
    // rename 到目标路径
}
```
如果不想引入 `tempfile` crate，可以用 `std::fs::File` + 手动生成随机临时文件名。

---

## Agent 2: Tool System 更新

### 你需要修改的文件
- `packages/core/src/tools/types.rs` — 更新 ToolHandler 枚举
- `packages/core/src/tools/registry.rs` — 更新工具注册
- `packages/core/src/tools/runner.rs` — 更新 ToolRunner

### 你需要阅读的参考文件（只读）
- `packages/core/src/memory/memory.rs` — 了解新的 MemoryManager API（见接口契约）

### 任务描述

将工具系统的记忆相关部分改造为支持双文件记忆和子字符串匹配的 replace/remove 操作。

#### 2.1 types.rs 改动

在 `ToolHandler` 枚举中：
- **移除** `MemoryOrganize`
- **新增** `MemoryReplace`、`MemoryRemove`

最终枚举:
```rust
pub enum ToolHandler {
    FileRead, FileWrite, Shell, Grep, Glob, Git,
    WebFetch, WebSearch,
    Remember,       // 保留，行为变更
    MemoryReplace,  // 新增
    MemoryRemove,   // 新增
    RecallArchive,
}
```

#### 2.2 registry.rs 改动

更新 `register_default_tools()`：

**Remember 工具（行为变更）：**
- 描述更新为: `"Save a note to persistent memory. Use target='memory' for agent notes (environment, project facts, learnings) or target='user' for user profile (preferences, communication style, habits). Do NOT use for secrets."`
- 新增 `"memory"` 类别

**新增 memory_replace 工具：**
```rust
ToolDescriptor {
    name: "memory_replace".to_string(),
    description: "Replace an existing memory entry. Uses substring matching via old_text to locate the entry. If multiple entries match, an error is returned — provide a more specific old_text.".to_string(),
    required_permissions: vec!["memory_write".to_string()],
    handler: ToolHandler::MemoryReplace,
}
```

**新增 memory_remove 工具：**
```rust
ToolDescriptor {
    name: "memory_remove".to_string(),
    description: "Remove a memory entry. Uses substring matching via old_text to locate the entry. If multiple entries match, an error is returned — provide a more specific old_text.".to_string(),
    required_permissions: vec!["memory_write".to_string()],
    handler: ToolHandler::MemoryRemove,
}
```

**移除 memory_organize 工具**

**更新 categories：**
- 移除 `"memory_write"` 单独的 category
- 新增 `"memory"` category: `["remember", "memory_replace", "memory_remove", "recall_archive"]`

#### 2.3 runner.rs 改动

##### 构造函数变更：
```rust
pub struct ToolRunner {
    memory_manager: MemoryManager,  // 不再 Option
    recall_archive: Option<RecallArchive>,
}

impl ToolRunner {
    pub fn new(memory_manager: MemoryManager, recall_archive: Option<RecallArchive>) -> Self {
        Self { memory_manager, recall_archive }
    }
}
```

##### execute 方法更新：
将 `ToolHandler::MemoryOrganize => ...` 替换为：
```rust
ToolHandler::MemoryReplace => self.execute_memory_replace(&call).await,
ToolHandler::MemoryRemove => self.execute_memory_remove(&call).await,
```

##### execute_remember 行为变更：
- 参数: `note` (保持), 新增 `target` (可选，默认 `"memory"`)
- 根据 `target` 选择 `MemoryTarget::Memory` 或 `MemoryTarget::User`
- 调用 `self.memory_manager.add(target, note)`
- 返回包含使用率信息的 JSON 字符串结果

```rust
async fn execute_remember(&self, call: &ToolCall) -> ToolResult {
    let note = /* ... */;
    let target_str = call.arguments.get("target")
        .and_then(|v| v.as_str())
        .unwrap_or("memory");
    let target = match target_str {
        "user" => MemoryTarget::User,
        _ => MemoryTarget::Memory,
    };
    
    let result = self.memory_manager.add(target, note);
    if result.ok {
        ToolResult::Success(result.message)
    } else {
        ToolResult::Error(ToolError::new("memory_error", result.message))
    }
}
```

##### 新增 execute_memory_replace：
- 参数: `target` (默认 `"memory"`), `old_text`, `content`
- 调用 `self.memory_manager.replace(target, old_text, content)`
- 返回 JSON 结果

##### 新增 execute_memory_remove：
- 参数: `target` (默认 `"memory"`), `old_text`
- 调用 `self.memory_manager.remove(target, old_text)`
- 返回 JSON 结果

##### 移除 execute_memory_organize

##### Default 实现更新：
```rust
impl Default for ToolRunner {
    fn default() -> Self {
        // 使用临时路径，仅用于测试
        Self::new(MemoryManager::new(Path::new("/tmp/.mimo")), None)
    }
}
```

##### 移除的内容：
- `MemoryEntry` 的 import（不再需要）
- `execute_memory_organize` 方法
- `ToolHandler::MemoryOrganize` 分支

---

## Agent 3: CLI main.rs 集成改动

### 你需要修改的文件
- `packages/apps/cli/src/main.rs`

### 你需要阅读的参考文件（只读）
- 参见接口契约章节中 Agent 1 和 Agent 2 提供的公共 API

### 任务描述

将记忆注入方式从"注入到第一条用户消息"改为"注入为 system prompt 冻结快照"，并更新工具定义。

#### 3.1 tool_to_definition 函数更新

**remember 工具 schema（增加 target 参数）：**
```rust
"remember" => serde_json::json!({
    "type": "object",
    "properties": {
        "note": {"type": "string", "description": "The note to save. One sentence or short paragraph."},
        "target": {"type": "string", "enum": ["memory", "user"], "description": "Which memory store: 'memory' for agent notes (environment, project facts, learnings) or 'user' for user profile (preferences, habits, communication style). Default: 'memory'"}
    },
    "required": ["note"]
}),
```

**新增 memory_replace schema：**
```rust
"memory_replace" => serde_json::json!({
    "type": "object",
    "properties": {
        "target": {"type": "string", "enum": ["memory", "user"], "description": "Which memory store. Default: 'memory'"},
        "old_text": {"type": "string", "description": "A unique substring of the entry to replace. Must match exactly one entry."},
        "content": {"type": "string", "description": "The new content for the entry"}
    },
    "required": ["old_text", "content"]
}),
```

**新增 memory_remove schema：**
```rust
"memory_remove" => serde_json::json!({
    "type": "object",
    "properties": {
        "target": {"type": "string", "enum": ["memory", "user"], "description": "Which memory store. Default: 'memory'"},
        "old_text": {"type": "string", "description": "A unique substring of the entry to remove. Must match exactly one entry."}
    },
    "required": ["old_text"]
}),
```

**移除 `memory_organize` 分支**（从 match 中删除）

#### 3.2 MemoryManager 初始化变更

在 `interactive_cli()` 函数中：

**旧代码：**
```rust
let memory_enabled = std::env::var("MIMO_MEMORY").is_ok();
let memory_manager = MemoryManager::new(&storage_path, memory_enabled);
```

**新代码：**
```rust
let mut memory_manager = MemoryManager::new(&storage_path);
// 会话开始时拍冻结快照
memory_manager.take_snapshot();
```

同样更新 `debug_repl()` 函数中的初始化。

#### 3.3 ToolRunner 初始化变更

**旧代码：**
```rust
let tool_runner = Arc::new(ToolRunner::new(Some(memory_manager), recall_archive));
```

**新代码：**
```rust
let tool_runner = Arc::new(ToolRunner::new(memory_manager, recall_archive));
```

> ⚠️ 注意：由于 memory_manager 需要同时被 interactive_cli 直接使用（获取 system prompt block）和被 ToolRunner 使用，你需要使用 `Arc<RefCell<MemoryManager>>` 或让 `ToolRunner` 暴露 `memory_manager()` 访问方法。

**推荐方案：** 给 `ToolRunner` 新增一个方法：
```rust
pub fn memory_manager(&self) -> &MemoryManager {
    &self.memory_manager
}
```
这样 `interactive_cli` 可以通过 `tool_runner.memory_manager()` 获取快照。

#### 3.4 System Prompt 注入方式变更（核心改动）

在 `interactive_cli()` 的主循环中，找到以下**旧代码**（约第 487-510 行）：

```rust
// 旧代码：每次都重新读取，注入到第一条 user 消息
let memory_block = if memory_enabled {
    let mm = MemoryManager::new(&storage_path, true);
    mm.compose_block()
} else {
    None
};

session.add_message(MessageRole::User, input.to_string());
session_manager.borrow_mut().save_session(&session)?;

let mut messages: Vec<ProviderMessage> = session.messages.iter()
    .map(|m| ProviderMessage { ... })
    .collect();

if let Some(block) = memory_block {
    if let Some(first) = messages.first_mut() {
        first.content = format!("{}\n\n{}", block, first.content);
    }
}
```

**改为：**

```rust
session.add_message(MessageRole::User, input.to_string());
session_manager.borrow_mut().save_session(&session)?;

let mut messages: Vec<ProviderMessage> = Vec::new();

// 冻结快照：只在消息列表最前面注入一次 system prompt
if let Some(memory_block) = tool_runner.memory_manager().get_system_prompt_block() {
    let system_content = format!(
        "你是一个智能编程助手 MiMo，基于 MiMo V2.5 Pro 模型。\n\
         你可以使用工具来完成用户的请求。\n\
         你的记忆在下方显示，这些记忆在会话之间持久保存。\n\
         使用 remember 工具保存新记忆，使用 memory_replace 更新条目，使用 memory_remove 删除条目。\n\n{}",
        memory_block
    );
    messages.push(ProviderMessage {
        role: ProviderMessageRole::System,
        content: system_content,
        name: None,
        tool_calls: None,
    });
}

// 追加对话历史
for m in &session.messages {
    messages.push(ProviderMessage {
        role: convert_role(&m.role),
        content: m.content.clone(),
        name: None,
        tool_calls: None,
    });
}
```

**关键区别：**
- 旧方式：`<user_memory>` XML 块注入到**第一条用户消息**内容前
- 新方式：记忆块作为独立的 **system 消息**插入到消息列表最前面
- 新方式：快照在会话开始时拍一次，整个会话不再变（`take_snapshot()` 只在上层调用一次）
- 不再每次循环重新创建 `MemoryManager` 实例

#### 3.5 移除的内容

- 删除 `memory_enabled` 变量及相关 UI 文本（`/help` 中的开关提示、`interactive_cli` 中的 `memory_info` 变量等）
- 删除快捷记忆命令 `#<note>`（第 465-475 行）—— 现在由 Agent 通过 `remember` 工具自行管理
- 删除 `/memory` 命令（第 460-463 行）和 `handle_memory_command` 函数 —— 记忆管理现在完全由 Agent 通过工具操作，不再需要用户手动命令
- 更新 `print_help` 函数，移除 `memory_enabled` 参数和记忆相关的手动命令文档
- 在 `debug_repl()` 函数中做同样的 system prompt 注入改造

#### 3.6 import 更新

```rust
// 移除: MemoryManager 的直接 import（如果通过 ToolRunner 访问）
// 新增: 无需额外 import
use mimo_core::{
    SessionManager, ToolRegistry, SubAgentManager,
    RecallArchive,
    session::{MessageRole, Session},
    tools::{ToolCall as CoreToolCall, ToolResult as CoreToolResult, ToolRunner},
};
```

> 注意：`MemoryManager` 不再需要直接出现在 main.rs 的 import 中（通过 ToolRunner 间接使用），除非你选择共享引用的方案。选择最简洁的方式。

---

## 🔄 Agent 执行顺序建议

1. **Agent 1 先执行** — 必须先完成 MemoryManager 的新 API，其他 Agent 才能引用
2. **Agent 2 接着执行** — 依赖 Agent 1 的 `MemoryTarget`、`MemoryOpResult` 等类型
3. **Agent 3 最后执行** — 依赖 Agent 1 的快照 API 和 Agent 2 的工具 API

---

## ✅ 验证清单（每个 Agent 完成后自检）

- [ ] `cargo build -p mimo-core` 通过（Agent 1、2）
- [ ] `cargo build -p mimo-cli` 通过（Agent 3）
- [ ] 没有引入新的 `unwrap()` 调用（除明确安全的地方）
- [ ] 所有新增方法都有错误处理（不 panic）
- [ ] 字符限制使用 `chars().count()` 而非 `len()`（对 Unicode 友好）
- [ ] 原子写入确保不产生损坏的中间状态
- [ ] 文件不存在时优雅降级（返回空/None 而非报错）
