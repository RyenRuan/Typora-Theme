# Ryen Mermaid 多场景回归测试

这份测试稿保留上一版本的基准结构，并增加多组标准 Mermaid 流程图，用于检查布局、分支、子图、回路、长文本和多图加载。

## 1. 基准流程：并行评估与三条返工回路

```mermaid
flowchart LR
    S([开始]) --> R[收集需求]
    R --> RD{需求完整?}
    RD -->|否| RF[补充信息<br/>返回：收集需求]
    RD -->|是| PLAN[方案设计]

    subgraph PARALLEL[并行评估]
        direction TB
        UI[UI 原型]
        TECH[技术评审]
    end

    PLAN --> UI
    PLAN --> TECH
    UI --> REVIEW[综合评审]
    TECH --> REVIEW

    REVIEW --> AD{评审通过?}
    AD -->|否| AF[修改方案<br/>返回：方案设计]
    AD -->|是| DEV[开发实施]

    DEV --> TD{测试通过?}
    TD -->|否| TF[修复问题<br/>返回：开发实施]
    TD -->|是| PUB[发布上线]
    PUB --> ARCHIVE[复盘归档]
    ARCHIVE --> END([结束])

    class R,PLAN,UI,TECH,REVIEW,DEV,ARCHIVE ryen-process;
    class RD,AD,TD ryen-decision;
    class S,PUB,END ryen-success;
    class RF,AF,TF ryen-rework;
```

## 2. 上下布局：审批分流与超时回退

```mermaid
flowchart TD
    A([提交申请]) --> B[自动校验资料]
    B --> C{资料是否完整?}
    C -->|否| D[通知补充资料<br/>返回：自动校验资料]
    C -->|是| E[进入人工审核]
    E --> F{风险等级}
    F -->|低| G[自动批准]
    F -->|中| H[主管复核]
    F -->|高| I[风控委员会评审]
    H --> J{复核通过?}
    I --> J
    J -->|否| K[重新提交说明<br/>返回：进入人工审核]
    J -->|是| G
    G --> L[生成审批记录]
    L --> M([完成])

    class B,E,H,I,L ryen-process;
    class C,F,J ryen-decision;
    class A,G,M ryen-success;
    class D,K ryen-rework;
```

## 3. 左右布局：两个并行工作区汇合

```mermaid
flowchart LR
    START([需求进入]) --> SPLIT{是否需要双线评估?}

    subgraph PRODUCT[产品工作区]
        direction TB
        P1[需求拆解]
        P2[交互原型]
        P3[产品验收]
        P1 --> P2
        P2 --> P3
    end

    subgraph ENGINEERING[工程工作区]
        direction TB
        E1[技术预研]
        E2[接口设计]
        E3[工程评审]
        E1 --> E2
        E2 --> E3
    end

    SPLIT -->|是| P1
    SPLIT -->|是| E1
    SPLIT -->|否| QUICK[快速方案]
    P3 --> MERGE[合并评审结果]
    E3 --> MERGE
    QUICK --> MERGE
    MERGE --> PASS{可以立项?}
    PASS -->|否| REWORK[调整范围<br/>返回：需求拆解]
    PASS -->|是| END([进入排期])

    class P1,P2,P3,E1,E2,E3,QUICK,MERGE ryen-process;
    class SPLIT,PASS ryen-decision;
    class START,END ryen-success;
    class REWORK ryen-rework;
```

## 4. 多分支测试：发布、灰度与回滚

```mermaid
flowchart TB
    S([构建完成]) --> T[执行自动化测试]
    T --> Q{全部通过?}
    Q -->|否| FIX[修复失败用例<br/>返回：执行自动化测试]
    Q -->|是| PKG[生成候选版本]
    PKG --> CANARY[小流量灰度]
    CANARY --> METRIC{核心指标正常?}
    METRIC -->|否| ROLLBACK[回滚并分析<br/>返回：生成候选版本]
    METRIC -->|是| EXPAND[扩大灰度范围]
    EXPAND --> FEEDBACK{收到严重反馈?}
    FEEDBACK -->|是| HOTFIX[准备热修复<br/>返回：执行自动化测试]
    FEEDBACK -->|否| RELEASE[全量发布]
    RELEASE --> MONITOR[持续监控]
    MONITOR --> DONE([发布完成])

    class T,PKG,CANARY,EXPAND,MONITOR ryen-process;
    class Q,METRIC,FEEDBACK ryen-decision;
    class S,RELEASE,DONE ryen-success;
    class FIX,ROLLBACK,HOTFIX ryen-rework;
```

## 5. 长文本与蛇形换行压力测试

```mermaid
flowchart LR
    N1([开始]) --> N2[收集来自客户端、服务端与运营后台的原始需求]
    N2 --> N3[识别必须保留的业务事实和不可变约束]
    N3 --> N4{信息是否足以形成可执行方案?}
    N4 -->|否| N5[补充缺失的验收条件<br/>返回：收集来自客户端、服务端与运营后台的原始需求]
    N4 -->|是| N6[拆分为界面、数据、接口和异常处理任务]
    N6 --> N7[评估依赖关系与潜在阻塞项]
    N7 --> N8[生成第一版实施顺序]
    N8 --> N9{整体风险是否可接受?}
    N9 -->|否| N10[缩小首期范围<br/>返回：拆分为界面、数据、接口和异常处理任务]
    N9 -->|是| N11[进入开发与联调]
    N11 --> N12[执行功能测试与视觉回归]
    N12 --> N13{验收结果是否满足发布标准?}
    N13 -->|否| N14[定位差异并修正实现<br/>返回：进入开发与联调]
    N13 -->|是| N15[准备发布说明与回滚预案]
    N15 --> N16[部署到目标环境]
    N16 --> N17[观察关键指标和错误日志]
    N17 --> N18([完成])

    class N2,N3,N6,N7,N8,N11,N12,N15,N16,N17 ryen-process;
    class N4,N9,N13 ryen-decision;
    class N1,N18 ryen-success;
    class N5,N10,N14 ryen-rework;
```

## 6. `graph` 别名与虚线依赖

```mermaid
graph LR
    A([任务创建]) --> B[准备输入]
    B --> C[主处理流程]
    B -.-> D[可选质量检查]
    C --> E{处理成功?}
    D -.-> E
    E -->|否| F[修正输入<br/>返回：准备输入]
    E -->|是| G[保存结果]
    G --> H([结束])

    class B,C,D,G ryen-process;
    class E ryen-decision;
    class A,H ryen-success;
    class F ryen-rework;
```
