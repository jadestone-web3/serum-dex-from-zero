# Step2: 余额管理 + 撤单功能（本地模拟）

本 demo 实现了一个最小化的订单簿撮合引擎，支持用户资产余额管理和订单的撤单，便于理解 DEX 的核心业务流程。

## 🆕 相比 Step01 的新增功能

| 功能 | Step01 | Step02 | 说明 |
|------|--------|--------|------|
| 基础撮合 | ✅ | ✅ | 买单/卖单自动撮合 |
| 订单ID | ❌ | ✅ | 唯一订单标识符 |
| 余额管理 | ❌ | ✅ | 主币和报价币余额 |
| 撤单功能 | ❌ | ✅ | 撤销未成交订单 |
| 余额校验 | ❌ | ✅ | 下单前检查余额 |
| 冻结机制 | ❌ | ✅ | 下单时冻结相应资产 |

## 🔧 核心改进

### 1. 订单ID系统
```rust
pub struct Order {
    pub id: u64,       // 唯一订单ID，自增
    pub owner: String, // 订单所有者
    pub side: Side,    // 买 or 卖
    pub price: u64,    // 价格
    pub quantity: u64, // 剩余数量
}
```

### 2. 用户余额管理
```rust
pub struct UserBalance {
    pub base: u64,  // 主币余额（如SOL）
    pub quote: u64, // 报价币余额（如USDC）
}
```

### 3. 余额校验机制
- **买单**：检查报价币余额是否足够（价格 × 数量）
- **卖单**：检查主币余额是否足够（数量）

### 4. 资产冻结机制
- 下单时立即冻结相应资产
- 撮合成功后结算
- 未成交部分返还给用户

### 5. 撤单功能
```rust
pub fn cancel_order(&mut self, user: &str, order_id: u64) -> bool
```
- 支持按订单ID撤销订单
- 自动返还冻结的资产
- 验证订单所有权

## 📁 项目结构

```
step02_orderbook_balance_cancel/
├── Cargo.toml              # 项目配置
├── Cargo.lock              # 依赖锁定
├── README.md               # 项目说明
└── src/
    ├── lib.rs              # 库入口点
    ├── main.rs             # 可执行程序入口
    └── openbook.rs         # 订单簿核心逻辑
```

## 🚀 快速开始

### 编译和运行
```bash
# 编译项目
cargo build

# 运行演示程序
cargo run
```

### 运行结果示例
```
用户 A 充值：主币 100，报价币 2000
用户 B 充值：主币 50，报价币 1000
买单部分未成交，剩余数量 10 进入订单簿，订单ID=1
撮合成交: 卖家:B 买家:A 价:10 数量:5
撤销买单，返还报价币 50，订单ID=1
买单簿: []
卖单簿: []
用户 A 主币余额:105 报价币余额:2050
用户 B 主币余额:45 报价币余额:1050
```

## 🧪 测试用法

### 1. 基础功能测试

#### 余额管理测试
```rust
let mut book = OrderBook::new();

// 用户充值
book.deposit("Alice", 100, 1000);  // 主币100，报价币1000
book.deposit("Bob", 50, 500);      // 主币50，报价币500

// 查看余额
book.print_balances();
```

#### 下单测试
```rust
// 买单：需要足够的报价币
let bid_id = book.place_order("Alice", Side::Bid, 10, 5);
// 冻结：50个报价币 (10 * 5)

// 卖单：需要足够的主币
let ask_id = book.place_order("Bob", Side::Ask, 10, 3);
// 冻结：3个主币
```

#### 撤单测试
```rust
// 撤销订单并返还冻结的资产
if let Some(id) = bid_id {
    book.cancel_order("Alice", id);
    // 返还：50个报价币
}
```

### 2. 完整交易流程测试

```rust
let mut book = OrderBook::new();

// 1. 用户充值
book.deposit("Trader1", 100, 2000);
book.deposit("Trader2", 50, 1000);

// 2. 下单
let order1 = book.place_order("Trader1", Side::Bid, 10, 10);
let order2 = book.place_order("Trader2", Side::Ask, 10, 5);

// 3. 查看撮合结果
book.print_book();
book.print_balances();

// 4. 撤单
if let Some(id) = order1 {
    book.cancel_order("Trader1", id);
}
```

### 3. 边界情况测试

#### 余额不足测试
```rust
let mut book = OrderBook::new();
book.deposit("User", 10, 50);

// 尝试下单超过余额
let result = book.place_order("User", Side::Bid, 10, 10);
// 预期：下单失败，余额不足
assert!(result.is_none());
```

#### 撤单权限测试
```rust
// 尝试撤销不属于自己的订单
let success = book.cancel_order("UserA", 999);
// 预期：撤单失败
assert!(!success);
```

## 🔍 核心API说明

### OrderBook 主要方法

| 方法 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `new()` | 无 | `OrderBook` | 创建新的订单簿 |
| `deposit()` | `user, base, quote` | 无 | 用户充值 |
| `place_order()` | `owner, side, price, quantity` | `Option<u64>` | 下单，返回订单ID |
| `cancel_order()` | `user, order_id` | `bool` | 撤单，返回是否成功 |
| `print_book()` | 无 | 无 | 打印订单簿状态 |
| `print_balances()` | 无 | 无 | 打印所有用户余额 |

### 数据结构

#### Side 枚举
```rust
pub enum Side {
    Bid, // 买单
    Ask, // 卖单
}
```

#### Order 结构体
```rust
pub struct Order {
    pub id: u64,       // 唯一订单ID
    pub owner: String, // 订单所有者
    pub side: Side,    // 买卖方向
    pub price: u64,    // 价格
    pub quantity: u64, // 剩余数量
}
```

#### UserBalance 结构体
```rust
pub struct UserBalance {
    pub base: u64,  // 主币余额
    pub quote: u64, // 报价币余额
}
```

## ⚠️ 重要说明

### 链上开发注意事项
- 当前实现使用 `HashMap` 模拟用户余额，仅用于本地演示
- 实际链上开发应使用区块链账户模型管理余额
- 所有余额变更需要通过链上指令和权限校验
- 订单状态应持久化到链上存储

### 安全考虑
- 实际部署时需要严格的权限控制
- 余额操作需要原子性保证
- 订单ID生成需要防重放攻击
- 价格和数量需要溢出检查

## 🎯 下一步计划

1. **添加单元测试和集成测试**
2. **实现更高效的订单簿数据结构**
3. **添加价格精度和数量精度控制**
4. **实现订单历史记录**
5. **添加手续费机制**
6. **实现限价单和市价单**
7. **添加订单簿深度查询**

## 📚 相关资源

- [Step01: 最小化订单簿](../step01_minimal_market/)
- [Rust 官方文档](https://doc.rust-lang.org/)
- [Solana 开发文档](https://docs.solana.com/)
