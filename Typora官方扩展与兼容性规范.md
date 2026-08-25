# Typora 官方扩展与 Markdown 兼容性规范

> 用途：作为 `Ryen-Custom` 的长期维护准则，说明哪些是 Typora 官方支持的扩展方式，以及本定制包对图片、字体和跨编辑器兼容性的约定。  
> 最近核对：2026-08-18。本文是维护规范，不替代 [Typora本地定制记录.md](Typora本地定制记录.md) 中已经生效的本机配置。

## 1. 核心结论

- 不直接改 Typora 内置主题（例如 `github.css`、`whitey.css`）。它们会在 Typora 更新、重启或主题更新时被替换。
- 面向全部主题的补充样式，使用主题目录中的 `base.user.css`。
- 面向某个主题的补充样式，使用 `{主题文件名}.user.css`；文件名必须与该主题 CSS 的文件名部分完全一致，且区分大小写。
- 本包已经部署的 `ryen-*.css` 是我们自己维护的独立主题，因此可以继续在这些独立文件中维护；不需要为了迁移而立即改动现有主题。
- Markdown 图片默认使用“相对路径 + `assets/` 文件夹”。这是在 Typora、Obsidian、Nora、Git 仓库和常见预览环境之间风险最低的格式。

## 2. Typora 官方支持的 CSS 扩展方式

Typora 的官方文档说明，样式加载顺序为：

1. Typora 基础样式；
2. 当前主题的 CSS；
3. 主题目录中的 `base.user.css`；
4. 主题目录中的 `{当前主题}.user.css`。

因此，后加载的 `.user.css` 可在不修改主题原文件的情况下覆盖样式。

| 目标 | 推荐文件 | 示例 | 说明 |
|---|---|---|---|
| 所有主题共用的规则 | `base.user.css` | 正文全局字体回退 | 仅放确实适用于所有主题的规则。 |
| 一个主题的增量调整 | `{theme}.user.css` | `ryen-whitey.user.css` | `theme` 应为已选主题 CSS 的文件名部分。 |
| 自己完整维护的主题 | 自己的主题 CSS | `ryen-whitey.css` | 本包现有的 5 个 `Ryen ...` 主题属于这一类。 |
| Typora 内置主题 | 不直接编辑 | `github.css` | 由 Typora 管理；更新时可能覆盖修改。 |

### 对当前 Ryen 定制包的执行规则

1. 保持 5 个 `ryen-*.css` 独立主题，不回写内置主题。
2. 以后若是“小范围覆盖”而不是重新维护主题，优先新增 `.user.css`，并在安装脚本中明确部署它。
3. 任何 `.user.css` 都要与目标主题同名：例如目标为 `ryen-whitey.css`，增量文件应为 `ryen-whitey.user.css`，不是 `whitey.user.css`。
4. 修改后重启 Typora，并在“主题”菜单中确认选中相应的 `Ryen ...` 主题。

## 3. 字体维护规范

- CSS 中的字体名称必须使用字体文件的**内部字体族名**，不能只根据文件名猜测。
- 当前定制字体的首选字体族为 `consolaslxgw`；后面必须保留合适的系统回退字体，避免在未安装字体的设备上出现不可读或错位。
- 新设备需要运行本包的安装程序，完成当前 Windows 用户范围内的字体安装；单纯复制 Typora 安装目录不会完成字体注册。
- 更换字体前应检查内部字体族名、文件哈希、中文字符和代码字符显示，再修改 CSS 与本机记录。

## 4. Markdown 图片与 Git 仓库规范

### 默认格式：相对路径

为一个 Markdown 文档建立同级 `assets/` 文件夹，图片引用写成：

```md
![流程示意](assets/flow-overview.png)
```

示例结构：

```text
README.md
assets/
  flow-overview.png
```

提交 Git 时应同时提交 Markdown 文件和 `assets/` 中被引用的图片。这样仓库内、Typora 以及多数 Markdown 编辑器都可按文档位置解析图片；不要写本机绝对路径，例如 `E:\\Pictures\\...`。

### Base64 / `data:image` 的边界

```md
![](data:image/png;base64,...)
```

这种写法可被部分本地编辑器渲染，但不能作为跨编辑器或 GitHub 预览的兼容性承诺：不同产品、版本和网页安全策略可能拒绝或过滤 `data:` URL；同时文档会明显增大，Git 差异也难以审阅。

所以，本包的默认策略是不把 Base64 内嵌图片用于需要共享、提交 Git 或长期维护的 Markdown。仅当文件必须单文件分发，并且已经在目标软件与目标版本中实测可用时，才可例外使用。

### 编辑器兼容性结论

| 场景 | 推荐做法 | Base64 是否作为默认方案 |
|---|---|---|
| Typora 本地写作 | 相对路径 + `assets/` | 否 |
| Obsidian / Nora 等其他编辑器 | 相对路径 + `assets/`，在目标版本实测 | 否 |
| Git 仓库 README、文档站或代码评审 | 相对路径 + 已提交的图片 | 否 |
| 必须单文件离线交付 | 先在接收方工具验证，再评估内嵌 | 仅例外 |

## 5. 源码与可维护边界

Typora 的编辑器核心是商业闭源软件。GitHub 上 `typora` 组织公开的主要是主题、文档、语言资源或辅助项目，不应把这些仓库视为应用程序核心源码。

对本定制包而言，长期可维护的边界是：主题 CSS、用户 CSS、字体、图片目录结构、安装脚本和本说明文档；不修改 Typora 的核心资源文件，也不依赖未公开的内部实现。

## 6. 每次调整后的最小检查

1. 确认没有修改内置主题文件。
2. 重启 Typora，确认选择的是 `Ryen ...` 主题或预期的目标主题。
3. 检查正文、代码块、表格、中文和英文混排。
4. 新增图片时，在 Typora 与目标编辑器各打开一次；如需提交 Git，再确认仓库预览。
5. 若修改安装包内文件，同步更新 `manifest.sha256` 和 [Typora本地定制记录.md](Typora本地定制记录.md)（仅在实际生效配置发生变化时）。

## 7. 依据与参考

- [Typora Support：Add Custom CSS](https://support.typora.io/Add-Custom-CSS/)：`base.user.css`、`{theme}.user.css` 的加载顺序、命名要求，以及不直接修改主题文件的原因。（本页最近更新于 2026-07-19，核对日期见本文顶部。）
- [Typora Support：About Themes](https://support.typora.io/About-Themes/)：主题安装与编写入口。
- [GitHub Docs：Basic writing and formatting syntax](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax)：GitHub Markdown 图片与格式语法参考。
- [Typora GitHub organization](https://github.com/orgs/typora/repositories)：公开仓库范围参考；它不代表 Typora 核心编辑器已开源。

