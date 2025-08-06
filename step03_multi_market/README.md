# Step03: 多市场（Multi-Market）订单簿撮合引擎

本章节在 step02（单市场的订单簿撮合、余额管理、撤单）基础上，进一步扩展支持**多市场（Multi-Market）**，即支持多个不同的交易对（如 SOL/USDC、BTC/USDT）独立运行。

---

## 本章节新增内容

- **多市场支持**：可动态创建多个市场，每个市场拥有独立的订单簿、用户余额、订单管理与撮合引擎。
- **市场隔离**：用户的充值、下单、撤单等操作需指定市场（market），每个市场内资金与订单完全隔离，互不影响。
- **结构设计更贴近 Serum DEX**：所有命名方式、结构体划分尽量与 [Serum DEX](https://github.com/project-serum/serum-dex/blob/master/dex/src/state.rs) 的 Market/Order/OrderBook/MarketState 体系保持一致，便于后续链上迁移与对比。

---

## 核心设计

### 数据结构与模块划分

- **Markets**  
  多市场管理器，维护所有市场的状态（HashMap\<市场名, MarketState\>）

- **MarketState**  
  单一市场的订单簿和用户余额池，包含：
  - `bids`/`asks`：买/卖单簿（Vec\<Order\>）
  - `balances`：用户余额表（HashMap\<用户名, UserBalance\>）
  - `next_order_id`：本市场下单自增ID

- **Order**  
  单个订单，包含唯一ID、用户、方向、价格、数量等。

- **UserBalance**  
  用户在某市场的主币余额和报价币余额。

- **Side**  
  订单方向（Bid/Ask）。

### 主要流程

1. **创建市场**：create_market(&mut self, market: &str) 。
可动态注册新市场（如 "SOL/USDC"），每市场独立订单簿、余额池。

2. **充值**：deposit(&mut self, market: &str, user: &str, base: u64, quote: u64)。
用户向指定市场充值主币和报价币。

3. **下单**：place_order(&mut self, market: &str, user: &str, side: Side, price: u64, quantity: u64)。
用户在指定市场挂买单/卖单，自动完成撮合，剩余部分进入对应订单簿。

4. **撤单**：cancel_order(&mut self, market: &str, user: &str, order_id: u64)。
用户可按订单ID撤销挂单，未成交部分返还余额。

5. **查询**：print_market_book(&self, market: &str)。
print_balances(&self, market: &str)
可分别查询不同市场的订单簿和用户余额。

---

## 如何运行

1. **准备**
   - 确保已安装 Rust 工具链
   - 推荐使用标准 cargo 项目结构（即 main.rs 放在 src/ 下）

2. **运行**
   - 若为 cargo 项目，直接运行：
     ```bash
     cargo run
     ```
   - 若只有 main.rs，可用 rustc 临时编译：
     ```bash
     rustc main.rs -o main
     ./main
     ```

---

## 与 step02 及 Serum DEX 的对比

- **step02**：仅支持单一市场，所有用户/订单/余额均在同一池内。
- **step03**：支持多个市场，结构更贴近实际 DEX 产品设计。
- **Serum DEX**：链上实现，每个 Market 是链上独立账户，并通过 PDA、Token Program 管理资产。step03 的本地模拟结构与 Serum 的 Market/OrderBook/UserBalance 体系高度对齐，便于未来迁移和理解。

---

## 扩展建议

- 支持全局订单ID或每市场自增ID
- 可加入市场创建权限、市场参数（如最小价格单位）
- 后续可扩展手续费、撮合历史等功能

---

> 本 demo 仅为本地业务逻辑模拟，实际链上开发需结合 Solana 的账户模型与 Token Program 进行资产托管和权限管理。