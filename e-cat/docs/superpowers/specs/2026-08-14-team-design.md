# e-cat 团队协作设计 — 矩阵式 8+1 团队

**日期:** 2026-08-14
**版本:** v2.3.5
**状态:** 已获用户批准（分节确认：角色清单 / 协作机制 / 落地步骤）

## 背景

e-cat 是 55 个 crate 的 Rust 微服务框架（对标 go-kratos），已过生态规划 v3 全部缺口
（v2.3.3 落地），处于成熟维护期。近期工作节奏为"深度审计 → 修复 → 补测试 → 同步文档"
循环。团队需覆盖四类工作：审计+修复、新功能开发、日常维护、工程化建设。

## 决策记录

| 问题 | 决策 | 理由 |
|------|------|------|
| 团队结构 | 矩阵式（方案 A），任务驱动组队 | 四类工作并存，纯流水线浪费空闲岗，分级主管 lead 成瓶颈 |
| 规模 | 8 名 agent + 1 名 lead（主会话） | 用户选定 8-9 人 |
| 落地形式 | 规划文档 → 用户确认 → 建队 | 用户选定 |
| 首个任务 | 深度审计 | 延续项目历史节奏 |

## 团队角色（8 名 agent）

| # | 角色 | 职责 | 主要产物 |
|---|------|------|----------|
| 1 | researcher-audit | 深度审计：bug、一致性、API 破坏性变更、覆盖率缺口；复现实测 | `docs/audit-report-*.md` |
| 2 | researcher-special | 专项审计：安全（依赖 CVE、TLS/mTLS、注入）、性能热点、bench 数据 | 专项审计章节 |
| 3 | architect | 新功能/新 crate 设计：API 形状、trait 边界、proto 变更、生态规划草案 | 设计文档 |
| 4 | coder-core | 核心域实现：transport/middleware/config/registry/metrics/tracing/auth/tls/events | 代码 + 单测 |
| 5 | coder-data | 数据域实现：14 个 data-* 后端、mq-*、graphql/openapi | 代码 + 单测 |
| 6 | tester | 测试补强、回归验证、ecat-testing mock、ecat-bench 基准 | 测试代码 + 报告 |
| 7 | reviewer | 正确性/风格/clippy 闸门、安全复查；批准后才算完成 | 评审结论 |
| 8 | docs-ops | 文档同步（README×2/CHANGELOG/教程/图片水印）；CI（GitHub Actions）、Helm/Dockerfile、版本发布三处同步 | 文档 + CI 配置 |

分工原则：
- coder 按域拆两个（核心 vs 数据），避免同时改 axum 层与数据驱动时上下文打架
- 两个 researcher 区分主审计与专项审计，对应多轮审计节奏
- docs-ops 合并文档+工程化（低负载轮转岗）；后续繁忙可拆为两岗

## 协作机制

### 拓扑：矩阵式，任务驱动组队

- lead 只做编排（任务分解、组队、验收、处理阻塞），不写代码
- 组内用 SendMessage 直连（遵循 CLAUDE.md Agent Comms 模式），组间不通信

### 任务 → 组队映射

| 任务类型 | 组队 | 流程 |
|----------|------|------|
| 深度审计（首个任务） | researcher-audit + researcher-special 并行 → tester 复核 → reviewer 确认 | 产出审计报告，发现的问题登记为任务 |
| 修复问题 | 按域 coder-core / coder-data → tester 回归 → reviewer 放行 | 串行 pipeline |
| 新功能/新 crate | architect 设计 → 对应 coder 实现 → tester → reviewer → docs-ops 同步 | 串行 pipeline |
| 维护同步 | coder（按域）+ docs-ops | 直接执行 |
| 工程化 | docs-ops + tester | 直接执行 |

## 质量闸门

1. `cargo build` + 相关 crate 的 `cargo test` 全绿（含全量 workspace 测试，覆盖 mTLS 竞态修复）
2. `clippy` 零告警
3. reviewer 批准后才算完成，才能进入文档同步
4. 版本发布需 lead 批准：CHANGELOG + Cargo.toml 版本 + Helm appVersion 三处同步

## 并发边界

- 同时最多跑 2 条任务线（如审计线 + 开发线），避免 tester/reviewer 成瓶颈
- 数据域与核心域改动可并行（crate 依赖解耦）

## 落地步骤

1. 本设计文档提交 git
2. 用户审阅文档并批准
3. 建队：TeamCreate + 一次性 spawn 8 个命名 agent（run_in_background: true）
4. 首个任务：深度审计（researcher-audit + researcher-special 并行）
