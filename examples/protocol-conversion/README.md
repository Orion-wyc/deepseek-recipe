# 协议转换示例

一个可直接运行的命令行示例，演示 `deepseek-recipe` 最基础的转换流程：同一段对话分别写成三种 API 协议（Chat Completions、Responses、Messages），再统一转换成 `Conversation`，最后渲染成 DeepSeek V4.1 的 prompt。

示例会打印解析出的模型版本、会话配置、渲染后的 prompt，以及按示例 prompt 长度解析出的 `max_tokens`。它不依赖模型、API key、tokenizer 或 OpenCV。

这个示例是**独立 crate**：`Cargo.toml` 里有自己的 `[workspace]` 表，依赖走 crates.io 上发布的 `deepseek-recipe` 0.2，不属于仓库根 workspace。可以整个目录复制到任意位置单独构建。

## 运行

进入示例目录：

```sh
cd examples/protocol-conversion
cargo run --locked
```

也可以从仓库根目录用 `--manifest-path` 指定：

```sh
cargo run --manifest-path examples/protocol-conversion/Cargo.toml --locked
```

指定只跑某一种协议：

```sh
cargo run --locked -- chat
cargo run --locked -- responses
cargo run --locked -- messages
```

不传参数时默认是 `all`，三种协议依次运行。

示例目录下有独立的 `Cargo.lock` 和 `target/`，和仓库根 workspace 的构建缓存互不影响。

## 演示内容

1. 把 JSON 反序列化为 `ChatCompletionRequest`、`ResponsesRequest` 或 `MessagesRequest`。
2. 调用 `ProtocolRequest::convert` 得到 `ConversationRequest`，其中包含会话、推理参数、解析参数和解析出的 `ModelVersion`。
3. 根据 `model_version` 选择 `DeepseekV4Encoding` 或 `DeepseekV41Encoding`，调用 `render_conversation`。
4. 用 `ConversationRequest::resolve_max_tokens` 解析 `max_tokens`。

三份输入描述的是同一个请求，因此转换后的会话相同，渲染出的 prompt 也相同。

## 输出示例

```text
=== Chat Completions ===
model version : DeepseekV41Flash
thinking mode : true
messages      : 2
tools         : 0
response fmt  : Text
stream        : false
temperature   : Some(1.0)
top_p         : Some(1.0)
max_tokens    : unset
prompt        : 234 bytes, 5 special tokens, 0 images
resolved      : max_tokens = 65536 at 16 prompt tokens

--- rendered prompt ---
<｜begin▁of▁sentence｜><｜System｜>Reasoning Effort: 75 (range 1-100, the higher the value, the more thorough the reasoning)

You are a helpful assistant.<｜User｜>What's the weather in Beijing today?<｜Assistant｜><think>
-----------------------
```

三种协议的输出应完全一致。

## 修改示例

- 请求内容：`src/main.rs` 顶部的 `CHAT_COMPLETIONS`、`RESPONSES`、`MESSAGES` 三个常量。
- 打印字段：`report` 函数。

几个与默认行为相关的点：

- 请求里不写 `thinking` 时默认开启思考模式，因此 prompt 会带上 `Reasoning Effort: 75`。
- 请求里不写 `max_tokens` 时，`resolve_max_tokens` 在思考模式下返回 65536。
- `model` 写 `DeepSeek-V4.1-Flash` 时解析为 `DeepseekV41Flash`，使用 V4.1 编码器。默认还接受 `DeepSeek-V4-Pro`，其他模型名需要通过 `ConversionOptions::with_model_alias` 注册。

## 依赖来源

默认使用 crates.io 上发布的 0.2：

```toml
deepseek-recipe = "0.2"
deepseek-recipe-encoding = "0.2"
```

如果想验证仓库里的当前源码，把这两行改成相对路径：

```toml
deepseek-recipe = { path = "../../deepseek-recipe" }
deepseek-recipe-encoding = { path = "../../deepseek-recipe-encoding" }
```

这样示例仍然独立于根 workspace，只是依赖的 crate 换成了本地路径；相对路径以本目录为基准，复制到别处时记得一起改。

## 下一步

- 把 prompt 编码成 token IDs：[使用 tokenizer](../../docs/tokenizer.zh.md)。
- 把后端输出流式转换为协议响应：[流式响应](../../docs/streaming.zh.md)。
- 带图片解析和 mock 推理的完整 HTTP 服务：[server-rs](../../server-rs/README.md)。
