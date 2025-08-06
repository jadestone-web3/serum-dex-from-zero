use std::collections::HashMap;

// ========== 订单方向 ==========
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Side {
    Bid,
    Ask,
}

// ========== 单个订单 ==========
#[derive(Debug, Clone)]
pub struct Order {
    pub id: u64,
    pub owner: String,
    pub side: Side,
    pub price: u64,
    pub quantity: u64,
}

// ========== 用户余额 ==========
#[derive(Debug, Default, Clone)]
pub struct UserBalance {
    pub base: u64,  // 主币余额
    pub quote: u64, // 报价币余额
}

// ========== FeeReceiver（手续费池） ==========
#[derive(Debug, Default)]
pub struct FeeReceiver {
    pub collected_fee: u64, // 仅统计报价币手续费
}

// ========== Event（成交/撤单历史） ==========
#[derive(Debug, Clone)]
pub enum EventType {
    Fill,   // 成交
    Cancel, // 撤单
}

#[derive(Debug, Clone)]
pub struct Event {
    pub event_type: EventType,
    pub market: String,
    pub maker: Option<String>,
    pub taker: Option<String>,
    pub price: Option<u64>,
    pub quantity: u64,
    pub fee: u64,
    pub order_id: u64,
    pub timestamp: u64,
}

// ========== MarketState（市场状态） ==========
#[derive(Debug, Default)]
pub struct MarketState {
    pub bids: Vec<Order>,
    pub asks: Vec<Order>,
    pub next_order_id: u64,
    pub balances: HashMap<String, UserBalance>,
    pub fee_receiver: FeeReceiver,
    pub event_queue: Vec<Event>,
}

/*
说明：
- Fee 机制：每笔撮合成交自动收取手续费（报价币），归 fee_receiver，余额和事件历史均已反映。
- Event Queue：每次成交和撤单都会生成历史事件，结构参考 Serum DEX 的 FillEvent/OutEvent，并可查询。
- 所有命名、结构尽量与 Serum DEX 保持一致，便于迁移链上。
*/

impl MarketState {
    /// 用户充值
    pub fn deposit(&mut self, user: &str, base: u64, quote: u64) {
        let bal = self.balances.entry(user.to_string()).or_default();
        bal.base += base;
        bal.quote += quote;
        println!(
            "用户 {} 在本市场充值：主币 {}，报价币 {}",
            user, base, quote
        );
    }

    /// 在指定市场下单（买/卖），支持手续费和时间戳
    pub fn place_order(
        &mut self,
        market: &str,      // 市场名，如 "SOL/USDC"
        owner: &str,       // 下单用户
        side: Side,        // 订单方向：买单(Bid) 或 卖单(Ask)
        price: u64,        // 下单价格（以报价币计价）
        mut quantity: u64, // 下单数量（主币数量，函数内会被多次修改）
        now: u64,          // 当前时间戳（如区块时间，撮合/历史用）
        fee_bps: u64,      // 手续费，单位为基点（1 bps = 0.01%）
    ) -> Option<u64> {
        let bal = self.balances.entry(owner.to_string()).or_default();
        match side {
            Side::Bid => {
                let needed_quote = price * quantity;
                if bal.quote < needed_quote {
                    println!("下单失败，用户 {} 报价币余额不足", owner);
                    return None;
                }
                // 链上不用担心，整个交易都是事务
                bal.quote -= needed_quote;
            }
            Side::Ask => {
                if bal.base < quantity {
                    println!("下单失败，用户 {} 主币余额不足", owner);
                    return None;
                }
                bal.base -= quantity;
            }
        }

        let order_id = self.next_order_id;
        self.next_order_id += 1;

        let mut order = Order {
            id: order_id,
            owner: owner.to_string(),
            side: side.clone(),
            price,
            quantity,
        };

        match side {
            Side::Bid => {
                while let Some(mut best_ask) = self.asks.first().cloned() {
                    if order.price >= best_ask.price && order.quantity > 0 {
                        let deal_qty = order.quantity.min(best_ask.quantity);
                        let deal_price = best_ask.price;

                        // 手续费，taker收（即发起撮合方）
                        let fee = deal_price * deal_qty * fee_bps / 10_000;
                        self.fee_receiver.collected_fee += fee;

                        // 买家（taker，当前order.owner）获得主币；卖家（maker）获得报价币
                        self.balances.get_mut(&order.owner).unwrap().base += deal_qty;
                        self.balances.get_mut(&best_ask.owner).unwrap().quote +=
                            deal_price * deal_qty - fee;

                        // 记录Fill Event
                        self.event_queue.push(Event {
                            event_type: EventType::Fill,
                            market: market.to_string(),
                            maker: Some(best_ask.owner.clone()),
                            taker: Some(order.owner.clone()),
                            price: Some(deal_price),
                            quantity: deal_qty,
                            fee,
                            order_id: order.id,
                            timestamp: now,
                        });

                        println!(
                            "撮合成交: 买家:{} 卖家:{} 价格:{} 数量:{} 手续费:{}",
                            order.owner, best_ask.owner, deal_price, deal_qty, fee
                        );
                        order.quantity -= deal_qty;
                        best_ask.quantity -= deal_qty;
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
                    let refund = order.price * order.quantity;
                    self.balances.get_mut(&order.owner).unwrap().quote += refund;
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
                        let deal_qty = order.quantity.min(best_bid.quantity);
                        let deal_price = best_bid.price;

                        let fee = deal_price * deal_qty * fee_bps / 10_000;
                        self.fee_receiver.collected_fee += fee;

                        // 卖家（taker，当前order.owner）获得报价币；买家（maker）获得主币
                        self.balances.get_mut(&order.owner).unwrap().quote +=
                            deal_price * deal_qty - fee;
                        self.balances.get_mut(&best_bid.owner).unwrap().base += deal_qty;

                        self.event_queue.push(Event {
                            event_type: EventType::Fill,
                            market: market.to_string(),
                            maker: Some(best_bid.owner.clone()),
                            taker: Some(order.owner.clone()),
                            price: Some(deal_price),
                            quantity: deal_qty,
                            fee,
                            order_id: order.id,
                            timestamp: now,
                        });

                        println!(
                            "撮合成交: 卖家:{} 买家:{} 价格:{} 数量:{} 手续费:{}",
                            order.owner, best_bid.owner, deal_price, deal_qty, fee
                        );
                        order.quantity -= deal_qty;
                        best_bid.quantity -= deal_qty;
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

    /// 撤单
    pub fn cancel_order(&mut self, market: &str, user: &str, order_id: u64, now: u64) -> bool {
        // 买单
        if let Some(pos) = self
            .bids
            .iter()
            .position(|o| o.id == order_id && o.owner == user)
        {
            let order = self.bids.remove(pos);
            let refund = order.price * order.quantity;
            self.balances.get_mut(user).unwrap().quote += refund;
            self.event_queue.push(Event {
                event_type: EventType::Cancel,
                market: market.to_string(),
                maker: None,
                taker: Some(user.to_string()),
                price: Some(order.price),
                quantity: order.quantity,
                fee: 0,
                order_id: order_id,
                timestamp: now,
            });
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
            self.event_queue.push(Event {
                event_type: EventType::Cancel,
                market: market.to_string(),
                maker: None,
                taker: Some(user.to_string()),
                price: Some(order.price),
                quantity: order.quantity,
                fee: 0,
                order_id: order_id,
                timestamp: now,
            });
            println!("撤销卖单，返还主币 {}，订单ID={}", order.quantity, order_id);
            return true;
        }
        println!("撤单失败，未找到属于用户 {} 的订单ID={}", user, order_id);
        false
    }

    pub fn print_book(&self) {
        println!("买单簿: {:?}", self.bids);
        println!("卖单簿: {:?}", self.asks);
    }

    pub fn print_balances(&self) {
        for (user, bal) in &self.balances {
            println!("用户 {} 主币:{} 报价币:{}", user, bal.base, bal.quote);
        }
    }

    pub fn print_fee_receiver(&self) {
        println!(
            "平台累计收取手续费(报价币): {}",
            self.fee_receiver.collected_fee
        );
    }

    pub fn print_events(&self) {
        println!("=== Event Queue（成交/撤单历史）===");
        for event in &self.event_queue {
            println!("{:?}", event);
        }
    }
}

// ========== 多市场管理 ==========
pub struct Markets {
    pub markets: HashMap<String, MarketState>,
}

impl Markets {
    pub fn new() -> Self {
        Self {
            markets: HashMap::new(),
        }
    }

    pub fn create_market(&mut self, market: &str) {
        self.markets
            .entry(market.to_string())
            .or_insert_with(MarketState::default);
        println!("新市场已创建: {}", market);
    }

    pub fn deposit(&mut self, market: &str, user: &str, base: u64, quote: u64) {
        if let Some(state) = self.markets.get_mut(market) {
            state.deposit(user, base, quote);
        } else {
            println!("市场 {} 不存在", market);
        }
    }

    pub fn place_order(
        &mut self,
        market: &str,
        owner: &str,
        side: Side,
        price: u64,
        quantity: u64,
        now: u64,
        fee_bps: u64,
    ) -> Option<u64> {
        if let Some(state) = self.markets.get_mut(market) {
            state.place_order(market, owner, side, price, quantity, now, fee_bps)
        } else {
            println!("市场 {} 不存在", market);
            None
        }
    }

    pub fn cancel_order(&mut self, market: &str, user: &str, order_id: u64, now: u64) -> bool {
        if let Some(state) = self.markets.get_mut(market) {
            state.cancel_order(market, user, order_id, now)
        } else {
            println!("市场 {} 不存在", market);
            false
        }
    }

    pub fn print_market_book(&self, market: &str) {
        if let Some(state) = self.markets.get(market) {
            println!("=== 市场 {} 订单簿 ===", market);
            state.print_book();
        } else {
            println!("市场 {} 不存在", market);
        }
    }

    pub fn print_market_balances(&self, market: &str) {
        if let Some(state) = self.markets.get(market) {
            println!("=== 市场 {} 用户余额 ===", market);
            state.print_balances();
        } else {
            println!("市场 {} 不存在", market);
        }
    }

    pub fn print_market_fee_receiver(&self, market: &str) {
        if let Some(state) = self.markets.get(market) {
            println!("=== 市场 {} 平台手续费 ===", market);
            state.print_fee_receiver();
        }
    }

    pub fn print_market_events(&self, market: &str) {
        if let Some(state) = self.markets.get(market) {
            println!("=== 市场 {} Event Queue ===", market);
            state.print_events();
        }
    }
}

// ========== 主程序 ==========
fn main() {
    let mut markets = Markets::new();
    let fee_bps = 30; // 0.3% (30 basis points)
    let mut now = 1_000_000_000u64; // 假定初始时间戳

    // 创建市场
    markets.create_market("SOL/USDC");
    markets.create_market("BTC/USDT");

    // 充值
    markets.deposit("SOL/USDC", "Alice", 100, 2000);
    markets.deposit("SOL/USDC", "Bob", 50, 1000);

    markets.deposit("BTC/USDT", "Carol", 10, 100000);
    markets.deposit("BTC/USDT", "Dave", 5, 80000);

    // 下单&撮合
    let alice_bid = markets.place_order("SOL/USDC", "Alice", Side::Bid, 10, 10, now, fee_bps);
    now += 1;
    let bob_ask = markets.place_order("SOL/USDC", "Bob", Side::Ask, 10, 5, now, fee_bps);
    now += 1;

    let carol_bid = markets.place_order("BTC/USDT", "Carol", Side::Bid, 20000, 2, now, fee_bps);
    now += 1;
    let dave_ask = markets.place_order("BTC/USDT", "Dave", Side::Ask, 19500, 3, now, fee_bps);
    now += 1;

    // 撤销剩余买单
    if let Some(id) = alice_bid {
        markets.cancel_order("SOL/USDC", "Alice", id, now);
        now += 1;
    }

    // 打印订单簿、余额、手续费池、历史事件
    markets.print_market_book("SOL/USDC");
    markets.print_market_balances("SOL/USDC");
    markets.print_market_fee_receiver("SOL/USDC");
    markets.print_market_events("SOL/USDC");

    markets.print_market_book("BTC/USDT");
    markets.print_market_balances("BTC/USDT");
    markets.print_market_fee_receiver("BTC/USDT");
    markets.print_market_events("BTC/USDT");
}
