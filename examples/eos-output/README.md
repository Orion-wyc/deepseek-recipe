# EOS 输出示例

这个示例说明一个容易混淆的边界：

- EOS 需要保留在内部，用于校验后端 token 数量和统计 usage；
- EOS 不应作为 Chat Completions、Responses 或 Messages 的正文返回。

示例直接调用 `deepseek-recipe` 的 `StreamProcessor`，模拟后端返回两类 token：

```text
[EOS, hello] + finish_reason=stop
[EOS]        + finish_reason=length
```

程序会分别运行：

1. **当前适配器式行为**：把 EOS 直接交给 Recipe；
2. **推荐行为**：先统计 EOS，再过滤 EOS，最后把剩余 token 交给 Recipe。

## 运行

```sh
cd examples/eos-output
cargo run --locked
```

示例使用仓库内的 V4.1 tokenizer：

```text
static/tokenizers/v41/tokenizer.json
```

## 观察结果

当前适配器式行为中：

- `[EOS, hello]` 的 EOS 会作为正文的一部分进入输出解析；
- `finish_reason=length` 且只有 EOS 时，EOS 也会进入正文；
- completion usage 仍应包含 EOS。

推荐行为中：

- 公开正文只包含 `hello` 或空字符串；
- 适配层仍按原始后端 token 数覆盖最终 usage。

示例中的“推荐行为”只演示了过滤动作，因此 Recipe 自己看到的 token 数会少一个 EOS，输出里的 `completion_tokens` 也会少一个。这正是适配层需要额外保存后端 `num_output_tokens` 并在最终响应中覆盖 usage 的原因；真实适配器不能直接使用过滤后的 Recipe usage。

Recipe 必须保留 EOS 的原因是它的 tokenizer 解码和输出解析需要保留特殊 token，以识别 thinking end 和 DSML 标记。但 EOS 本身不是公开正文，适配层需要在送入 `StreamProcessor` 前过滤它。

另外，程序会展示单 token frame 的尾 token 延迟：当前适配器每次缓存每个 frame 的最后一个 token，因此第一个普通 token 要等到下一个后端 frame 才会进入 Recipe。只缓存 EOS 可以避免这类额外延迟。

## 验证结果

在 `cargo run --locked` 下实测（V4.1 tokenizer，EOS ID = 1，`hello` = `[33310]`）：

| 场景 | 后端 token 数 | Recipe 输入 ID | 公开正文 | `completion_tokens` |
| --- | --- | --- | --- | --- |
| `[EOS, hello]` + `stop`，当前行为 | 2 | `[1, 33310]` | `<｜end▁of▁sentence｜>hello` | 2 |
| `[EOS]` + `length`，当前行为 | 1 | `[1]` | `<｜end▁of▁sentence｜>` | 1 |
| `[EOS, hello]` + `stop`，过滤 EOS | 2 | `[33310]` | `hello` | 1 |

**结论：** EOS 直连 Recipe 时确实会作为公开正文输出，且 `finish_reason=length` 下正文会只剩一个 EOS 字符串；在适配层提前过滤 EOS 后正文正确，但 Recipe 统计的 `completion_tokens` 比后端少 1，因此适配层必须用后端 `num_output_tokens` 覆盖最终 usage。两个结论都与前面的分析一致，示例行为符合预期。
