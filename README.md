# Typora Theme

Ryen 的 Typora 便携定制主题、脚本与本机 Mermaid 渲染器。本目录可以随 Typora 安装目录一起复制，但它不会修改 `resources` 下的 Typora 核心文件。

## 开发与部署边界

- 唯一开发仓库：`D:\Agent\ToolsDevelop\Typora-Theme`
- 当前稳定安装目录：`E:\Editor\Typora\Typora 1.9.5`
- 主题、脚本和 Mermaid 渲染器只在开发仓库中修改并提交。
- 通过构建、自动测试和人工验收后，才运行安装脚本部署到稳定安装目录。
- `node_modules`、Rust `target`、本机部署状态和临时回退目录不进入 Git。

## 新设备使用

1. 建议先正常安装并启动一次 Typora。
2. 复制整个 `Ryen-Custom` 文件夹到新设备。
3. 双击 `安装Typora定制.cmd`。
4. 在 Typora 的“编辑 -> 智能标点”中关闭“智能引号”和“渲染时转换”。
5. 重启 Typora，在“主题”菜单中选择一个 `Ryen ...` 主题，再测试字体、中英文引号和主题效果。

安装程序会自动完成：

- 安装合并字体到当前 Windows 用户。
- 以 `Ryen Github`、`Ryen Newsprint`、`Ryen Night`、`Ryen Pixyll`、`Ryen Whitey` 的名称复制 5 个独立主题及其字体资源。
- 部署仅对 Typora 生效的中文直角引号脚本。
- 安装 Typora 本机 Mermaid 渲染器：标准 `flowchart/graph LR、TD/TB` 由隔离 SVG 接管，其他 Mermaid 类型原生回退。
- 创建当前用户的 Windows 开机启动快捷方式。
- 启动引号映射服务。

## 中文输入效果

- 双引号键：`「｜」`
- 单引号键：`『｜』`
- 中文状态在 Typora 中粘贴时：`“”‘’` 自动转换为 `「」『』`。
- 粘贴后的系统剪贴板会自动恢复，不影响下一次在其他软件中粘贴。
- 支持 `Ctrl + V` 与 `Shift + Insert`；纯文本和网页富文本均会转换。
- ASCII 英文引号 `"'` 不转换，避免影响代码、JSON 等内容。
- `｜` 表示光标位置。
- 英文状态保持 Typora 原有引号行为。
- 其他软件不受影响。

## 卸载

双击 `卸载Typora定制.cmd`。

卸载脚本会停止并移除引号映射和启动项，同时移除 5 个 `Ryen ...` 主题和本机 Mermaid 渲染器；若安装前存在同名主题或 `window.html`，则恢复其备份。字体和备份文件会保留。

## 注意

- 仅复制 Typora 安装目录不能自动注册字体或创建当前用户的启动项，因此新设备必须运行一次安装脚本。
- 安装程序不会修改 `github.css`、`whitey.css` 等 Typora 内置主题；内置主题可能在 Typora 重启或更新时被自动替换。
- Mermaid 注入只修改本机 Typora 的 `resources\window.html` 和运行文件，安装前会生成 `.bak-before-ryen-mermaid`；Markdown 文件仍是标准 Mermaid。
- 不建议把整个 `profile.data` 复制到不同版本的 Typora。
- Typora 授权和用户偏好不属于本定制包，需要在新设备上单独配置。

## 维护文档

- `Typora本地定制记录.md`：本机已生效配置、历史变更与恢复说明。
- `Typora官方扩展与兼容性规范.md`：Typora 官方扩展方式和 Markdown 跨编辑器策略。
- `AI-Mermaid绘图规范.md`：AI 生成流程图时的布局、语义、配色和兼容性规则。
- `mermaid-renderer\README.md`：本机替换渲染层的架构、支持范围、构建、安装和回滚说明。
