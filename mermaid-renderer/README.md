# Ryen Mermaid 本机渲染器

这是 Typora 1.9.5 的本机 Mermaid 替换渲染层。Markdown 中仍然保存标准 Mermaid 代码，其他编辑器、GitHub 和其他设备看到的内容不会改变。

本目录就是本项目的唯一工作目录：源码、构建脚本、测试和构建产物都放在
`D:\Agent\ToolsDevelop\Typora-Theme\mermaid-renderer`。稳定运行文件部署到
`E:\Editor\Typora\Typora 1.9.5\resources\ryen-mermaid-renderer`，不再直接修改安装目录中的源码副本。

## 设计

```text
标准 Mermaid 源码
  -> Typora 注入脚本（TypeScript 打包）
  -> Web Worker
  -> Rust/WASM 解析、分层与布局
  -> Shadow DOM 中的隔离 SVG
```

Rust 只负责纯计算和 SVG 字符串生成；TypeScript 负责 Typora DOM 监听、缓存、Worker 通信和原生回退。

## 当前接管范围

- `flowchart LR`
- `flowchart TD` / `flowchart TB`
- `graph LR`
- `graph TD` / `graph TB`
- 节点、判断节点、开始/结束节点、简单 `subgraph`、节点标签、带标签连线、虚线连线和回环通道。
- 支持 `class` 样式映射：`ryen-process`、`ryen-decision`、`ryen-success`、`ryen-rework`。

当前内置视觉皮肤采用深色节点表面、紫/橙/绿语义描边和轻量发光；页面背景仍由 Typora 主题控制。判断节点使用按宽度自适应高度的对称菱形，避免因文字长度产生扁斜变形。

对于较长的 `flowchart LR`，布局器会先估算自然宽度，超过 1400 个 SVG 单位时自动按层换行，并让相邻行反向排列以保持流程连续；换行不依赖节点名称或固定示例。返工节点会根据其入边决策节点进入上下侧车道，并从主流程层高计算中剥离，避免分支占位把下一行无谓推远；同一决策的多个返工节点会在车道内自动堆叠。

蛇形换行后的右向左连线会使用源节点左端口和目标节点右端口，避免连接线穿过节点内容；水平连线不使用可能裁掉零高度路径的发光滤镜。

不支持的 Mermaid 类型或语法会保留 Typora 原生渲染，不会清空图块。首期节点上限为 120 个，避免编辑大图时卡顿。

## 性能保护

- Worker 后台执行布局，输入防抖 180 ms。
- WASM 只初始化一次。
- 按源码哈希缓存最近 80 个结果。
- 只有渲染成功后才隐藏原生 SVG；解析失败立即回退。
- 本地烟测 100 次复杂流程图平均约 0.177 ms/次（不包含首次 Worker 启动，机器负载不同会有小幅波动）。

## 构建

Rust 工具链归档在 `E:\SDKs\Rust`，用户环境变量 `CARGO_HOME` 和 `RUSTUP_HOME` 已指向该目录。构建命令：

```powershell
& 'C:\Users\Ryen\AppData\Local\Programs\PowerShell\7\pwsh.exe' `
  -ExecutionPolicy Bypass -File '.\scripts\Build.ps1'
```

构建会在当前目录执行 WASM 编译、TypeScript 类型检查和 Web 打包，并把最新的三个运行文件写入当前目录的 `dist`。

pnpm 依赖缓存统一固定到 `E:\SDKs\pnpm-store`（用户级 `PNPM_CONFIG_STORE_DIR` 与 pnpm 全局配置同步）；构建脚本也会显式使用该目录，不在项目所在盘根目录创建 `.pnpm-store`。

## 安装与卸载

关闭 Typora 后运行定制包根目录的 `安装Typora定制.cmd`。安装脚本只接受 Typora `1.9.5 / b20995f3`，会：

1. 备份 `resources\window.html`。
2. 在 `resources\ryen-mermaid-renderer` 放入 `main.js`、`worker.js`、`renderer.wasm`。
3. 加入带版本标记的单个脚本标签。
4. 写入 `install-manifest.json`。

卸载会依据标记恢复 `window.html`，移除运行目录，保留备份文件；不会修改任何 Markdown 源码和主题 CSS。

Typora 更新后不要盲目重复补丁。先确认 `package.json` 版本和 `releaseId`，版本不匹配时安装脚本会拒绝执行。
