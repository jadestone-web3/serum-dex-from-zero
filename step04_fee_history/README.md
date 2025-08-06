# Step04: 手续费（Fee）与成交历史（Event Queue）机制

本章节基于 Step03 的多市场订单簿撮合系统，进一步引入**手续费（Fee）**和**成交历史（Event/Event Queue）**两个核心 DEX 功能模块。所有命名和结构设计尽量与 [Serum DEX](https://github.com/project-serum/serum-dex) 保持一致，便于后续链上迁移和理解。

---

## 新增功能简介

### 1. 手续费（Fee）

#### 重要性
- **手续费是 DEX 的核心盈利机制**。通过对每笔撮合成交收取手续费，平台可维持运营并为流动性提供者和推荐人返佣。
- **安全和透明**：所有手续费的收取与归集需公开、可追溯，防止资金流失和作弊。

#### 原理
- 在订单撮合成交时，系统自动收取成交额的固定比例（如 0.3%）作为手续费。
- 手续费归集到市场 fee_receiver（或平台账户），用户余额变更时已自动扣除。
- 相关命名和流程参考 Serum DEX 的 fee 处理方式。

---

### 2. 成交历史（Event Queue）

#### 重要性
- **成交历史（Event/Fill/OutEvent）是 DEX 透明性和可审计性的基础**。用户和前端可实时获取所有成交、撤单等事件，支持做 K 线、实时行情等业务。
- **链上兼容**：Serum DEX 在链上通过 Event Queue 账户存储所有撮合和撤单事件，前端和服务端均可顺序消费。

#### 原理
- 撮合成交或撤单时，自动生成一个 Event（事件），包括事件类型、成交双方、价格、数量、手续费、时间戳等。
- 每个 MarketState 内部维护 event_queue（事件队列），可查询和导出。
- 事件结构设计参考 Serum 的 `Event`, `FillEvent`, `OutEvent` 设计。

---

## 主要结构与术语（对齐 Serum DEX）

- **Fee/FeeReceiver**：撮合手续费、手续费归集账户
- **Event/Event Queue**：撮合和撤单历史队列
- **FillEvent**：成交事件
- **OutEvent**：撤单事件
- 其它如 Market/MarketState/Order/Side 均保持与前述章节一致

---

## 使用与运行

1. 按 step03 方式初始化、创建市场、充值、下单和撤单
2. 每笔成交自动收手续费（可在 fee_receiver 查询累计手续费）
3. 每笔成交和撤单自动写入事件队列（可按市场查询 event queue）

---

## 与 Serum DEX 的关系

- **Fee 机制和 Event Queue 机制是 Serum DEX 链上合约的核心设计**，本地模拟有助于深入理解链上资产流转和事件队列原理，为后续合约开发和调试打基础。
- 通过对齐命名和结构，便于未来直接迁移到 Solana 合约或对比主网实现。

---

## 扩展建议

- 支持 maker/taker 不同费率、推荐人返佣等
- Event Queue 支持分页、按用户过滤等高阶功能

---

> 本 Demo 仅为本地业务逻辑模拟，实际链上开发需结合 Solana 的账户模型与 Token Program 进行资产托管和权限管理。所有结构和命名均向 Serum DEX 官方靠拢，便于学习和对接生态。