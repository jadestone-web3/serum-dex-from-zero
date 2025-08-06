## Serum DEX 的最核心功能是什么？

Serum DEX 的最核心本质是**一个链上订单簿撮合引擎**，用来撮合买家和卖家。
如果只保留最小骨架，最最核心的功能就是：

- 用户挂买单/卖单（下单）
- 系统撮合订单（撮合引擎）
- 撮合后资产转移（结算）

我们可以只用内存结构，先不考虑账户安全、PDA、权限、手续费、事件队列等所有复杂设计，把“一个市场的挂单和撮合”这件事用最简单的 Rust 代码实现出来。

## 项目结构
``` markdown
step01_minimal_market/
├── Cargo.toml              # 项目配置
├── Cargo.lock              # 依赖锁定
├── src/
│   ├── lib.rs              # 库入口，导出模块
│   ├── main.rs             # 可执行程序入口（简化版）
│   └── openbook.rs         # 订单簿核心逻辑 + 单元测试
├── tests/
│   └── integration_test.rs # 集成测试
├── target/                 # 编译输出（被.gitignore忽略）
├── README.md
└── README.zh.md
```

## 🧪 测试覆盖范围
### 单元测试 (6个)：
- ✅ 订单簿创建
- ✅ 买单下单
- ✅ 卖单下单
- ✅ 撮合逻辑
- ✅ 价格优先级
- ✅ 部分成交
### 集成测试 (3个)：
- ✅ 完整交易场景
- ✅ 边界情况测试
- ✅ 市价单模拟

 运行测试
 ```
 # 运行所有测试
cargo test

# 运行单元测试
cargo test --lib

# 运行集成测试
cargo test --test integration_test

# 运行主程序
cargo run
 ```