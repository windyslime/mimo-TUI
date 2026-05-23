# 任务 5: Chat 视图集成 MiMo 引擎

> 先阅读 `00-项目总览.md` 了解项目背景，然后执行本任务。

## 重要性

这是最核心的任务 —— 让 TUI 真正能与 MiMo 对话。

## 要求

### 1. 初始化流程

```
1. 读取 MIMO_API_KEY 环境变量
2. 创建 MimoProvider
3. 执行 health_check
4. 创建 SessionManager、ToolRegistry、ToolRunner、MemoryManager
5. 创建新会话
6. 显示状态（API 连接成功/失败、记忆开启/关闭）
```

### 2. 消息发送流程（用户按 Ctrl+Enter 后）

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

### 3. 异步处理

- 使用 `tokio` 异步运行时
- LLM 调用在异步任务中执行
- 通过 channel（`tokio::sync::mpsc`）将更新发送到 UI 线程
- UI 在等待响应时显示 "正在思考..." 或进度指示

### 4. 错误处理

- API 错误 → 在状态栏显示错误
- 网络超时 → 显示重试提示
- 工具执行失败 → 显示错误并追加到消息历史

### 5. 参考 CLI 实现

- `process_message` 函数的逻辑（`packages/apps/cli/src/main.rs` 第 287 行开始）
- `build_tools_from_registry` 工具注册
- 记忆块注入逻辑
- 工具调用循环

## 交付物

- 修改 `packages/apps/tui/src/views/chat.rs`：集成完整对话流程
- 支持异步消息流、工具调用、错误处理
- `cargo build` 编译通过
