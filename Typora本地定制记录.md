# Typora 本地定制记录

> 建档日期：2026-08-07  
> 当前 Typora：1.9.5  
> 安装目录：`E:\Editor\Typora`  
> 用途：记录本机 Typora 的主题样式、中文直角引号映射、原始备份和恢复方法。

## 1. 当前定制概览

| 项目 | 当前效果 | 生效范围 |
|---|---|---|
| 正文字体 | `consolaslxgw`（Consolas Nerd Font + 等宽霞鹜文楷） | 5 个 `Ryen ...` 独立主题 |
| 代码字体 | `consolaslxgw` 优先，保留旧合并字体、Cascadia Code、Consolas 作为回退 | 5 个 `Ryen ...` 独立主题 |
| 文档间距 | 正文行高、段落间距、标题和列表间距经过调整 | 5 个本地主题 |
| 表格样式 | 增加单元格留白、全部垂直居中，并按列数自适应宽度 | 5 个本地主题 |
| 中文双引号 | 中文状态按一次生成 `「｜」`，光标位于中间 | 仅 Typora 前台窗口 |
| 中文单引号 | 中文状态按一次生成 `『｜』`，光标位于中间 | 仅 Typora 前台窗口 |
| 粘贴中文弯引号 | 中文状态粘贴时将 `“”‘’` 转换为 `「」『』` | 仅 Typora 前台窗口 |
| 英文引号 | 保留 Typora 原有的 `"｜"`、`'｜'` 配对体验 | 仅 Typora |
| 其他软件 | 不进行任何引号映射 | 全部其他软件 |

`｜` 只用于表示光标位置，不会实际写入文档。

## 2. 变更时间线

### 2026-07-17：主题定制

- 将 5 个主题统一改为中英文合并字体。
- 调整正文行距和段落间距，使段落层级更明显。
- 增加表格单元格留白。
- 将表格所有单元格改为垂直居中。
- 根据表格列数自适应宽度和横向留白。
- 尝试过自动判断左对齐/居中，效果不理想后已撤回；当前未保留该规则。

### 2026-08-05：中文直角引号

- 关闭 Typora 自带的“智能引号”，避免英文状态被转换。
- 新增仅对 Typora 生效的中文输入状态检测和引号映射。
- 修复 Chromium 窗口无法通过传统输入法上下文识别中文状态的问题。
- 中文双引号改为 `「」`，中文单引号改为 `『』`。
- 后续改为一次补全一对符号，并将光标放在中间。
- 配置开机自动启动映射服务。

### 2026-08-07：状态核对与建档

- 确认映射服务仍在运行。
- 确认启动快捷方式、源脚本、主题原始备份和配置备份均存在。
- 建立本记录。

### 2026-08-13：粘贴文本引号转换

- 增加仅在 Typora 中文输入状态下生效的粘贴转换。
- 粘贴内容中的 `“”‘’` 自动转换为 `「」『』`。
- ASCII 英文引号不会转换，避免影响代码、JSON 等内容。
- 支持 `Ctrl + V` 和 `Shift + Insert`。
- 尽量保留网页复制内容的 HTML 格式，粘贴后自动恢复原剪贴板。

### 2026-08-14：独立主题和新字体名修复

- 更新为仓库当前字体 `consolaslxgw.ttf`，CSS 首选字体族改为 `consolaslxgw`。
- 不再覆盖 Typora 的 `github.css`、`whitey.css` 等内置主题。
- 5 个定制主题改以 `ryen-*.css` 独立部署，避免被 Typora 在重启或更新时替换。
- 安装时可迁移旧版附加在内置主题末尾的定制规则。

### 2026-08-18：正文区域加宽

- 5 个 `Ryen ...` 独立主题的正文区域统一改为：桌面端最大宽度 `1400px`，同时保留窗口两侧 `48px` 的最小留白。
- 在较窄窗口（宽度不超过 `760px`）时，自动将两侧留白收至 `18px`，避免横向滚动或内容贴边。
- 表格继续跟随正文区域宽度；5 列及以上表格可使用全部正文宽度。

### 2026-08-21：Mermaid 通用皮肤与 AI 绘图规范

- 5 个 `Ryen ...` 独立主题加入 Mermaid 通用皮肤，统一字体、默认节点、连线、标签、子图、时序图参与者和备注样式。
- 浅色主题使用柔和蓝色节点；`Ryen Night` 使用独立深色配色。
- 代码中显式设置的语义节点类别和连线样式尽量保留。
- 新增 `AI-Mermaid绘图规范.md`，规定布局选择、节点语义、颜色分类、连线和兼容性要求。
- 根据实际渲染结果，取消外围底板，并彻底撤销 SVG 宽高及整体缩放覆盖；图表尺寸交由 Mermaid/Typora 原生计算，只保留节点、连线和标签的视觉皮肤，避免粘贴后渲染失败。
- 根据实际查看结果，将 Mermaid 的 `2rem` 放大方案缩小 50%，恢复为 Typora 原生 `1rem` 字号；不使用固定 SVG 宽高、`zoom` 或 `transform`。
- 5 个主题的默认 Mermaid 流程图正式采用 C「柔彩糖果」风格：处理节点为柔紫、判断节点为暖黄、圆形/椭圆节点为薄荷绿，连线与子图使用淡紫体系，并保留轻量阴影和大圆角。
- 默认皮肤只匹配没有自定义类别的节点；图内显式 `classDef` 与 `linkStyle` 保持优先，便于单张图独立换色。
- 修正 C 风格在 Typora 生成 SVG 上的选择器命中范围，避免 Mermaid 内联默认样式令节点退回方框和默认连线。
- 明确主题不控制 Mermaid 自动布局；复杂流程采用无环横向主图，失败节点在文字中标明返回目标，避免多个回环与跨子图连线造成节点乱序和飞线。
- 增加 `ryen-process`、`ryen-decision`、`ryen-success`、`ryen-rework` 四个稳定语义类；Mermaid 代码只分配类名，C 风格的颜色、圆角和阴影仍由主题统一维护。
- 修复本机 SVG 箭头标记被节点填充遮挡的问题；箭头改为固定小尺寸并将尖端贴齐节点边界，避免按线宽放大造成三角形过大。
- 修复水平 Mermaid 连线被零高度发光滤镜裁掉的问题；连线改为直接绘制高对比描边，避免节点之间出现空白。
- 修复蛇形换行后的反向连线端口；右向左流程改从源节点左侧连接到目标节点右侧，避免横线穿过下排节点文字。

## 3. 字体定制

### 字体文件

`C:\Users\Ryen\AppData\Local\Microsoft\Windows\Fonts\consolaslxgw.ttf`

- 文件大小：16,246,416 字节
- CSS 使用的首选字体族：`consolaslxgw`
- SHA-256：`AB1BEDF3109336AB7724EABCAA786192D3CA20935FEBA0BF02E6C3DEF026B057`

### 独立定制主题

主题目录：

`C:\Users\Ryen\AppData\Roaming\Typora\themes`

已部署：

- `ryen-github.css`
- `ryen-newsprint.css`
- `ryen-night.css`
- `ryen-pixyll.css`
- `ryen-whitey.css`

Typora 内置的 `github.css`、`newsprint.css`、`night.css`、`pixyll.css`、`whitey.css` 保持原样。

每个主题的自定义内容都放在 CSS 文件尾部，并带有以下注释标记：

- `Ryen custom font override`
- `Codex compact spacing override`
- `Codex table spacing override`
- `Codex table all-column alignment override`
- `Codex adaptive table presentation override`

### 主要样式参数

- 正文行高：`1.52`
- 段落上间距：`0.75em`
- 段落下间距：`0.95em`
- 标题行高：`1.35`
- 表格行高：`1.55`
- 普通单元格内边距：`0.55em 0.85em`
- 所有表格单元格：`vertical-align: middle`
- 少于 5 列的表格：增加左右留白
- 5 列及以上的表格：使用 100% 宽度并收紧单元格左右留白
- 表格第一列：保留最低宽度，减少内容过度挤压
- 正文最大宽度：`1400px`（桌面端）；默认两侧留白：`48px`
- Mermaid：使用 Typora 原生画布、`1rem` 字号与自动尺寸，默认采用 C「柔彩糖果」节点、连线和标签皮肤，并尊重图内 `classDef` / `linkStyle`

## 4. 中文直角引号功能

### 当前行为

| 输入环境 | 按键 | 结果 |
|---|---|---|
| Typora 中文状态 | `Shift + 引号键` | `「｜」` |
| Typora 中文状态 | 直接按引号键 | `『｜』` |
| Typora 英文状态 | `Shift + 引号键` | Typora 原生双引号 |
| Typora 英文状态 | 直接按引号键 | Typora 原生单引号 |
| Typora 中文状态 | `Ctrl + Shift + 引号键` | 原始 `"` |
| Typora 中文状态 | `Ctrl + 引号键` | 原始 `'` |
| Typora 中文状态 | `Ctrl + V` / `Shift + Insert` | 将剪贴板中的 `“”‘’` 转为 `「」『』` 后粘贴 |
| 其他软件 | 任意上述按键 | 完全不映射 |

### 实现文件

运行脚本：

`C:\Users\Ryen\AppData\Roaming\Typora\scripts\typora-corner-quotes.ps1`

可编辑源文件副本：

`C:\Users\Ryen\Documents\Codex\2026-08-05\files-mentioned-by-the-user-codex\work\typora-corner-quotes.ps1`

脚本 SHA-256：

`3579D534349ADFAB2111CBD2356C8BC58828F1275EF6FFAA8D274B5143891A43`

脚本通过前台进程名将范围严格限定为 `Typora.exe`，并读取 Windows 输入法转换状态来区分中文和英文模式。

### 开机启动

快捷方式：

`C:\Users\Ryen\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Startup\Typora Corner Quotes.lnk`

启动目标：

`C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe`

启动参数：

```text
-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "C:\Users\Ryen\AppData\Roaming\Typora\scripts\typora-corner-quotes.ps1"
```

### Typora 当前相关设置

配置文件：

`C:\Users\Ryen\AppData\Roaming\Typora\profile.data`

截至建档时：

```text
smartQuote = false
SmartyPantsOnRendering = false
smartDash = false
userQuotesArray = 「 | 」 | 『 | 』
```

`profile.data` 是 Typora 的整体配置文件，不建议跨版本或跨电脑直接覆盖。

## 5. 原始备份

### 主题原始备份

| 主题 | 原始备份 |
|---|---|
| GitHub | `C:\Users\Ryen\AppData\Roaming\Typora\themes\github.css.bak-20260717-120853` |
| Newsprint | `C:\Users\Ryen\AppData\Roaming\Typora\themes\newsprint.css.bak-20260717-121217` |
| Night | `C:\Users\Ryen\AppData\Roaming\Typora\themes\night.css.bak-20260717-121217` |
| Pixyll | `C:\Users\Ryen\AppData\Roaming\Typora\themes\pixyll.css.bak-20260717-121217` |
| Whitey | `C:\Users\Ryen\AppData\Roaming\Typora\themes\whitey.css.bak-20260717-121217` |

这些备份是修改前的原始主题文件，应长期保留。

### Typora 配置备份

`C:\Users\Ryen\AppData\Roaming\Typora\profile.data.bak-corner-quotes-20260805-110220`

- 文件大小：1,734 字节
- SHA-256：`B29D4BE9588BB14C757DEC2A51DC63CF2065CDBC33977CC865F873EF6FD1FC49`

该文件是完整配置快照，恢复它会同时恢复当时的其他 Typora 偏好设置。

## 6. 恢复方法

### 只恢复某个主题

1. 关闭 Typora，或先切换到另一个主题。
2. 保留对应的 `.bak-*` 原始备份。
3. 将原始备份复制一份并覆盖对应的 `.css` 文件。
4. 重新启动 Typora。

例如，恢复 GitHub 主题时，应使用：

`github.css.bak-20260717-120853` → `github.css`

不要直接重命名或删除原始备份，应该复制后覆盖。

### 暂停中文引号功能

最安全的临时停用方法是结束命令行中包含以下脚本路径的隐藏 PowerShell 进程：

`C:\Users\Ryen\AppData\Roaming\Typora\scripts\typora-corner-quotes.ps1`

下次登录 Windows 时，启动快捷方式会再次运行它。

### 永久停用中文引号功能

1. 先结束上述映射进程。
2. 将启动快捷方式移出 Windows 的 `Startup` 文件夹。
3. 保留脚本文件，方便以后恢复。

### 恢复引号修改前的 Typora 配置

1. 保存所有正在编辑的文档。
2. 完全退出 Typora。
3. 将 `profile.data.bak-corner-quotes-20260805-110220` 复制为 `profile.data`。
4. 重新启动 Typora。

注意：这会恢复完整配置快照，而不仅是引号设置。

## 7. 换电脑或重装后的重新部署

推荐按以下顺序操作：

1. 安装 Typora。
2. 安装合并字体文件。
3. 将 5 个主题 CSS 以 `ryen-*.css` 名称放入新电脑的 Typora 主题目录。
4. 在 Typora 中关闭“智能引号”和“渲染时转换”。
5. 将 `typora-corner-quotes.ps1` 放入新电脑的 Typora `scripts` 目录。
6. 按本记录中的目标和参数重新创建开机启动快捷方式。
7. 重启 Typora，在“主题”菜单中选择一个 `Ryen ...` 主题。
8. 启动脚本并进行中英文状态测试。

不要在不同 Typora 版本间直接复制整个 `profile.data`；优先通过设置界面重新设置相关选项。

## 8. 验证清单

### 主题

- 中文和英文均使用合并字体。
- 正文段落之间明显大于同一段内的行距。
- 表格单元格不拥挤。
- 表格所有列均垂直居中。
- 少列表格不过度拉伸，多列表格可以利用整行宽度。

### 引号

- 中文状态按一次双引号键，得到 `「｜」`。
- 中文状态按一次单引号键，得到 `『｜』`。
- 英文状态保留普通英文引号和原生配对。
- 在记事本、浏览器等其他软件中，引号键不受影响。

### 运行状态

状态文件：

`C:\Users\Ryen\AppData\Roaming\Typora\scripts\typora-corner-quotes.status.txt`

运行日志目录：

`C:\Users\Ryen\AppData\Roaming\Typora\scripts`

正常情况下，最新的 `stderr.log` 应为空文件。

## 9. 文件校验值

| 文件 | SHA-256 |
|---|---|
| `github.css` | `8CBC2B87C9EA27821BD903E2EFCF07F832B7464E1F0C9E75314D3F2398FD0DF6` |
| `newsprint.css` | `3C010069AA068BD306042B7725C0F1B095F53C5A742167ECD6B4800F408779CA` |
| `night.css` | `7CB80F41819A53D98FDF5E0F42393500A43DCCBA0F1B26BCBE217117F31BA95F` |
| `pixyll.css` | `1915A49AC779A830F282E1725BF357AB59687E5CFABCAC098D6B19F8FBDE8527` |
| `whitey.css` | `52852C66CCA2DF99F8F2B09424D3494DA39809DFEFB6B8C1F68711CBF724AB0E` |
| `consolaslxgw.ttf` | `AB1BEDF3109336AB7724EABCAA786192D3CA20935FEBA0BF02E6C3DEF026B057` |
| `typora-corner-quotes.ps1` | `3579D534349ADFAB2111CBD2356C8BC58828F1275EF6FFAA8D274B5143891A43` |

如果以后继续修改这些文件，校验值发生变化是正常的；届时应更新本记录。

## 10. 修改边界

当前定制涉及：

- 用户主题 CSS
- Typora 用户配置
- Typora 专用外部按键映射脚本
- Windows 开机启动快捷方式
- Typora 1.9.5 的 `resources\window.html` Mermaid 注入和 `resources\ryen-mermaid-renderer` 运行文件

Mermaid 注入只在本机 Typora 内生效，不改变 Markdown 中的标准代码。安装前备份为：

`E:\Editor\Typora\Typora 1.9.5\resources\window.html.bak-before-ryen-mermaid`

安装脚本会校验 Typora `1.9.5 / b20995f3`；Typora 更新后版本不匹配会拒绝补丁。卸载时仅恢复该备份和移除本机运行目录。

尚未修改 `E:\Editor\Typora\resources` 下的 Typora 打包程序、原生模块或授权相关文件。

## 11. 便携部署包

便携包位置：

`D:\Agent\ToolsDevelop\Typora-Theme`

该目录可以随 Typora 安装目录一起复制，包含：

- 当前 5 个修改后的主题及其字体资源
- 5 个 Typora 1.9.5 原始主题副本
- 合并字体文件
- 中文直角引号脚本
- 自动安装与卸载脚本
- `mermaid-renderer` Rust/WASM + TypeScript 源码和构建脚本
- 新设备使用说明
- `manifest.sha256` 文件完整性清单

当前便携包约 31.90 MiB，共 62 个受校验文件（保留旧字体文件用于回退；构建缓存目录不计入清单）。

新设备上双击：

`安装Typora定制.cmd`

即可把定制内容部署到当前 Windows 用户。由于字体注册、主题目录和开机启动项属于用户环境，所以仅复制文件夹还不够，仍需在新设备运行一次安装脚本。

便携包会以可回滚方式修改 `resources\window.html` 和增加本机 Mermaid 运行目录；不会修改 `app.asar`、授权、原生模块或 Markdown 文件。Typora 授权和其他用户偏好仍需在新设备单独配置。

## 12. Mermaid 本机替换渲染层

实现目录：

`D:\Agent\ToolsDevelop\Typora-Theme\mermaid-renderer`

运行文件部署目录：

`E:\Editor\Typora\Typora 1.9.5\resources\ryen-mermaid-renderer`

首版采用 Rust/WASM 做解析和布局，TypeScript 做 Worker 与 Typora 接入。渲染结果通过 Shadow DOM 和 SVG 内联样式隔离当前主题；标准 `flowchart/graph LR、TD/TB` 成功接管，`sequenceDiagram` 等不支持类型保持原生。复杂实机测试已验证：

- pnpm 依赖缓存统一固定到 `E:\SDKs\pnpm-store`，构建脚本不会再按项目所在盘符生成 `.pnpm-store`。
- Ryen Newsprint：自定义流程图成功接管。
- Ryen Night：切换主题后自定义节点配色保持不变。
- `sequenceDiagram`：保留 Typora 原生 Mermaid。
- 长 `flowchart LR` 超过 1400 个 SVG 单位时自动分层换行，相邻行反向排列；返工节点进入判断节点上下车道，不占主流程层高，同一决策的多个返工节点会自动堆叠。
- 100 次 WASM 复杂图烟测：平均 0.177 ms/次（不含首次 Worker 启动）。

## 13. 粘贴乱码根因与剪贴板修复（2026-09-15）

现象：从 AI 桌面应用复制表格粘贴到 Typora 后，单元格末尾汉字变成 `�?/td>` 之类的乱码。

根因：中文引号服务（`typora-corner-quotes.ps1`，运行于 Windows PowerShell 5.1）在粘贴时会重写剪贴板。而 .NET Framework 读写 `HTML Format`（CF_HTML）剪贴板格式时使用系统 ANSI 代码页（GBK），并非 CF_HTML 规范要求的 UTF-8：读入时中文先变成 GBK 错解字符串，写回时再按 GBK 编码，Chromium 内核的 Typora 按 UTF-8 解析后产生大量 U+FFFD，且头部字节偏移与内容错位，于是露出半截 `</td>` 标签。

修复：脚本内 HTML 格式的读取改为 Win32 `OpenClipboard`/`GetClipboardData` 直取原始字节并按 UTF-8 解码；写回时改用 `MemoryStream` 承载 UTF-8 字节，完全绕开 .NET 对该格式的 ANSI 转码。粘贴时快照与粘贴后还原两条路径均已覆盖。

验证：端到端测试在真实剪贴板上完成——转换后粘贴内容与快照还原内容均字节级完整（48 个 `</td>`、无 U+FFFD、偏移有效、引号转换与还原均正确）。该缺陷自 2026-08-05 引号服务上线即存在，仅影响含弯引号的富文本（HTML）粘贴。
