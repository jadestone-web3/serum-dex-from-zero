use std::collections::HashMap;

/*
注释说明：
- 本地结构体模拟链上账户、订单簿和余额。实际链上开发需将 MarketState、UserBalance 等设计为账户结构，并用 Solana/SPL Token 指令管理资产。
- 多市场由 Markets 统一管理，所有操作需指定市场名，与 Serum 的 “Market” 体系对齐。
- 订单ID每市场自增，防止不同市场订单ID冲突。
*/

/// 订单方向
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Side {
    Bid,
    Ask,
}

/// 单个订单结构体
#[derive(Debug, Clone)]
pub struct Order {
    pub id: u64,
    pub owner: String,
    pub side: Side,
    pub price: u64,
    pub quantity: u64,
}

/// 用户余额结构体（每市场独立）
#[derive(Debug, Default, Clone)]
pub struct UserBalance {
    pub base: u64,  // 主币余额（如 SOL/BTC）
    pub quote: u64, // 报价币余额（如 USDC/USDT）
}

/// 单一市场状态（订单簿、余额、订单ID自增器）
#[derive(Debug, Default)]
pub struct MarketState {
    pub bids: Vec<Order>,
    pub asks: Vec<Order>,
    pub next_order_id: u64,
    pub balances: HashMap<String, UserBalance>, // key: 用户名
}

impl MarketState {
    /// 用户充值（模拟链上充值，实际链上应为账户转账）
    pub fn deposit(&mut self, user: &str, base: u64, quote: u64) {
        let bal = self.balances.entry(user.to_string()).or_default();
        bal.base += base;
        bal.quote += quote;
        println!(
            "用户 {} 在本市场充值：主币 {}，报价币 {}",
            user, base, quote
        );
    }

    /// 下单，自动撮合，余额校验与变更
    pub fn place_order(
        &mut self,
        owner: &str,
        side: Side,
        price: u64,
        mut quantity: u64,
    ) -> Option<u64> {
        // 余额校验
        let bal = self.balances.entry(owner.to_string()).or_default();
        match side {
            Side::Bid => {
                let needed_quote = price * quantity;
                if bal.quote < needed_quote {
                    println!("下单失败，用户 {} 报价币余额不足", owner);
                    return None;
                }
                bal.quote -= needed_quote; // 挂单先全部冻结，未成交部分后返还
            }
            Side::Ask => {
                if bal.base < quantity {
                    println!("下单失败，用户 {} 主币余额不足", owner);
                    return None;
                }
                bal.base -= quantity; // 挂单先全部冻结，未成交部分后返还
            }
        }

        // 新订单
        let order_id = self.next_order_id;
        self.next_order_id += 1;

        let mut order = Order {
            id: order_id,
            owner: owner.to_string(),
            side: side.clone(),
            price,
            quantity,
        };

        // 撮合流程
        match side {
            Side::Bid => {
                while let Some(mut best_ask) = self.asks.first().cloned() {
                    if order.price >= best_ask.price && order.quantity > 0 {
                        let qty = order.quantity.min(best_ask.quantity);
                        // 结算
                        self.balances.get_mut(&order.owner).unwrap().base += qty;
                        self.balances.get_mut(&best_ask.owner).unwrap().quote +=
                            best_ask.price * qty;
                        println!(
                            "撮合成交: 买家:{} 卖家:{} 价格:{} 数量:{}",
                            order.owner, best_ask.owner, best_ask.price, qty
                        );
                        order.quantity -= qty;
                        best_ask.quantity -= qty;
                        if best_ask.quantity == 0 {
                            self.asks.remove(0);
                        } else {
                            self.asks[0] = best_ask;
                            break;
                        }
                    } else {
                        break;
                    }
                }
                if order.quantity > 0 {
                    // 未成交部分返还报价币
                    let refund = order.price * order.quantity;
                    self.balances.get_mut(&order.owner).unwrap().quote += refund;
                    // 剩余部分入订单簿
                    self.bids.push(order.clone());
                    self.bids.sort_by(|a, b| b.price.cmp(&a.price));
                    println!(
                        "买单部分未成交，剩余 {} 进入买单簿，订单ID={}",
                        order.quantity, order.id
                    );
                }
            }
            Side::Ask => {
                while let Some(mut best_bid) = self.bids.first().cloned() {
                    if order.price <= best_bid.price && order.quantity > 0 {
                        let qty = order.quantity.min(best_bid.quantity);
                        self.balances.get_mut(&order.owner).unwrap().quote += best_bid.price * qty;
                        self.balances.get_mut(&best_bid.owner).unwrap().base += qty;
                        println!(
                            "撮合成交: 卖家:{} 买家:{} 价格:{} 数量:{}",
                            order.owner, best_bid.owner, best_bid.price, qty
                        );
                        order.quantity -= qty;
                        best_bid.quantity -= qty;
                        if best_bid.quantity == 0 {
                            self.bids.remove(0);
                        } else {
                            self.bids[0] = best_bid;
                            break;
                        }
                    } else {
                        break;
                    }
                }
                if order.quantity > 0 {
                    // 未成交部分返还主币
                    self.balances.get_mut(&order.owner).unwrap().base += order.quantity;
                    self.asks.push(order.clone());
                    self.asks.sort_by(|a, b| a.price.cmp(&b.price));
                    println!(
                        "卖单部分未成交，剩余 {} 进入卖单簿，订单ID={}",
                        order.quantity, order.id
                    );
                }
            }
        }
        Some(order_id)
    }

    /// 撤销订单
    pub fn cancel_order(&mut self, user: &str, order_id: u64) -> bool {
        // 买单
        if let Some(pos) = self
            .bids
            .iter()
            .position(|o| o.id == order_id && o.owner == user)
        {
            let order = self.bids.remove(pos);
            let refund = order.price * order.quantity;
            self.balances.get_mut(user).unwrap().quote += refund;
            println!("撤销买单，返还报价币 {}，订单ID={}", refund, order_id);
            return true;
        }
        // 卖单
        if let Some(pos) = self
            .asks
            .iter()
            .position(|o| o.id == order_id && o.owner == user)
        {
            let order = self.asks.remove(pos);
            self.balances.get_mut(user).unwrap().base += order.quantity;
            println!("撤销卖单，返还主币 {}，订单ID={}", order.quantity, order_id);
            return true;
        }
        println!("撤单失败，未找到属于用户 {} 的订单ID={}", user, order_id);
        false
    }

    /// 打印订单簿
    pub fn print_book(&self) {
        println!("买单簿: {:?}", self.bids);
        println!("卖单簿: {:?}", self.asks);
    }

    /// 打印所有用户余额
    pub fn print_balances(&self) {
        for (user, bal) in &self.balances {
            println!("用户 {} 主币:{} 报价币:{}", user, bal.base, bal.quote);
        }
    }
}

/// 多市场管理器
pub struct Markets {
    pub markets: HashMap<String, MarketState>, // key: market symbol，如 "SOL/USDC"
}

impl Markets {
    pub fn new() -> Self {
        Self {
            markets: HashMap::new(),
        }
    }

    /// 创建市场
    pub fn create_market(&mut self, market: &str) {
        self.markets
            .entry(market.to_string())
            .or_insert_with(MarketState::default);
        println!("新市场已创建: {}", market);
    }

    /// 用户充值到指定市场
    pub fn deposit(&mut self, market: &str, user: &str, base: u64, quote: u64) {
        if let Some(state) = self.markets.get_mut(market) {
            state.deposit(user, base, quote);
        } else {
            println!("市场 {} 不存在", market);
        }
    }

    /// 下单
    pub fn place_order(
        &mut self,
        market: &str,
        owner: &str,
        side: Side,
        price: u64,
        quantity: u64,
    ) -> Option<u64> {
        if let Some(state) = self.markets.get_mut(market) {
            state.place_order(owner, side, price, quantity)
        } else {
            println!("市场 {} 不存在", market);
            None
        }
    }

    /// 撤销订单
    pub fn cancel_order(&mut self, market: &str, user: &str, order_id: u64) -> bool {
        if let Some(state) = self.markets.get_mut(market) {
            state.cancel_order(user, order_id)
        } else {
            println!("市场 {} 不存在", market);
            false
        }
    }

    /// 打印指定市场订单簿
    pub fn print_market_book(&self, market: &str) {
        if let Some(state) = self.markets.get(market) {
            println!("=== 市场 {} 订单簿 ===", market);
            state.print_book();
        } else {
            println!("市场 {} 不存在", market);
        }
    }

    /// 打印指定市场所有用户余额
    pub fn print_market_balances(&self, market: &str) {
        if let Some(state) = self.markets.get(market) {
            println!("=== 市场 {} 用户余额 ===", market);
            state.print_balances();
        } else {
            println!("市场 {} 不存在", market);
        }
    }
}

fn main() {
    let mut markets = Markets::new();

    // 创建两个市场
    markets.create_market("SOL/USDC");
    markets.create_market("BTC/USDT");

    // 用户A、B在SOL/USDC市场充值
    markets.deposit("SOL/USDC", "Alice", 100, 2000);
    markets.deposit("SOL/USDC", "Bob", 50, 1000);

    // 用户C、D在BTC/USDT市场充值
    markets.deposit("BTC/USDT", "Carol", 10, 100000);
    markets.deposit("BTC/USDT", "Dave", 5, 80000);

    // Alice在SOL/USDC挂买单
    let alice_bid = markets.place_order("SOL/USDC", "Alice", Side::Bid, 10, 10);

    // Bob在SOL/USDC挂卖单，触发撮合
    let bob_ask = markets.place_order("SOL/USDC", "Bob", Side::Ask, 10, 5);

    // Carol在BTC/USDT挂买单
    let carol_bid = markets.place_order("BTC/USDT", "Carol", Side::Bid, 20000, 2);

    // Dave在BTC/USDT挂卖单，部分撮合
    let dave_ask = markets.place_order("BTC/USDT", "Dave", Side::Ask, 19500, 3);

    // Alice尝试撤销剩余买单（如果有）
    if let Some(id) = alice_bid {
        markets.cancel_order("SOL/USDC", "Alice", id);
    }

    // 打印订单簿和余额
    markets.print_market_book("SOL/USDC");
    markets.print_market_balances("SOL/USDC");
    markets.print_market_book("BTC/USDT");
    markets.print_market_balances("BTC/USDT");
}
