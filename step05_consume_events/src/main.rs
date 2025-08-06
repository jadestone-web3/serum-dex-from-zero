use std::collections::{HashMap, VecDeque};

/// 订单方向（买单/卖单）
/// Side is order side (Bid/Ask)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Side {
    /// 买单（出价买入）
    Bid,
    /// 卖单（挂出卖出）
    Ask,
}

/// 订单结构
#[derive(Debug, Clone)]
pub struct Order {
    /// 订单唯一ID
    pub id: u64,
    /// 持有者（用户名）
    pub owner: String,
    /// 订单方向（买/卖）
    pub side: Side,
    /// 挂单价格
    pub price: u64,
    /// 挂单数量
    pub quantity: u64,
    /// 订单过期时间戳（可选，Some(ts)则ts时刻后订单无效）
    pub expire_ts: Option<u64>,
}

/// 用户余额信息
#[derive(Debug, Default, Clone)]
pub struct UserBalance {
    /// 主币余额（如SOL/BTC/ETH等）
    pub base: u64,
    /// 报价币余额（如USDC/USDT等）
    pub quote: u64,
}

/// 平台手续费归集账户
#[derive(Debug, Default)]
pub struct FeeReceiver {
    /// 已累计收取的手续费（单位：报价币）
    pub collected_fee: u64,
}

/// 事件类型枚举（撮合/撤单/过期）
/// EventType describes the event kind in event queue.
#[derive(Debug, Clone)]
pub enum EventType {
    /// 成交事件（订单被撮合成交）
    Fill,
    /// 撤单事件（用户撤销订单）
    Cancel,
    /// 过期事件（订单到期自动撤销）
    Expire,
}

/// 事件队列中每条事件结构
#[derive(Debug, Clone)]
pub struct Event {
    /// 事件类型（成交/撤单/过期）
    pub event_type: EventType,
    /// 所属市场名（如 "SOL/USDC"）
    pub market: String,
    /// maker账户（撮合中的被动方，部分事件可为None）
    pub maker: Option<String>,
    /// taker账户（撮合中的主动方，部分事件可为None）
    pub taker: Option<String>,
    /// 成交价格（部分事件可为None）
    pub price: Option<u64>,
    /// 成交数量
    pub quantity: u64,
    /// 手续费（单位：报价币）
    pub fee: u64,
    /// 订单ID
    pub order_id: u64,
    /// 事件发生的时间戳
    pub timestamp: u64,
}

/// 市场事件队列
#[derive(Debug, Default)]
pub struct EventQueue {
    /// 事件列表（先进先出队列）
    pub events: VecDeque<Event>,
    /// 下一个事件序号（用于分配事件序号、便于指针管理）
    pub next_seq: u64,
    /// 每个consumer（如crank/前端）消费指针，记录该consumer已消费到第几个事件
    pub consumer_positions: HashMap<String, u64>,
}

impl EventQueue {
    /// 推入新事件
    pub fn push(&mut self, event: Event) {
        self.events.push_back(event);
        self.next_seq += 1;
    }

    /// 消费者批量消费事件，返回未消费事件并推进消费指针
    /// consumer: 消费者ID
    /// max_events: 本次最多消费的事件数
    pub fn consume_events(&mut self, consumer: &str, max_events: usize) -> Vec<Event> {
        let last_pos = self
            .consumer_positions
            .entry(consumer.to_string())
            .or_insert(0);
        let mut result = vec![];
        let total_events = self.events.len() as u64;
        let mut cnt = 0;
        while *last_pos < total_events && cnt < max_events {
            let idx = *last_pos as usize;
            if idx < self.events.len() {
                result.push(self.events[idx].clone());
                *last_pos += 1;
                cnt += 1;
            } else {
                break;
            }
        }
        result
    }
}

/// 单一市场状态
#[derive(Debug, Default)]
pub struct MarketState {
    /// 买单簿（降序按价格排列，价格高优先）
    pub bids: Vec<Order>,
    /// 卖单簿（升序按价格排列，价格低优先）
    pub asks: Vec<Order>,
    /// 下一个订单号（自增ID）
    pub next_order_id: u64,
    /// 用户余额表
    pub balances: HashMap<String, UserBalance>,
    /// 平台手续费账户
    pub fee_receiver: FeeReceiver,
    /// 事件队列
    pub event_queue: EventQueue,
}

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

    /// 清理所有已过期订单
    pub fn clean_expired_orders(&mut self, now: u64, market: &str) {
        // 买单
        self.bids.retain(|o| {
            let expired = o.expire_ts.map(|ts| ts <= now).unwrap_or(false);
            if expired {
                let refund = o.price * o.quantity;
                self.balances.get_mut(&o.owner).unwrap().quote += refund;
                self.event_queue.push(Event {
                    event_type: EventType::Expire,
                    market: market.to_string(),
                    maker: None,
                    taker: Some(o.owner.clone()),
                    price: Some(o.price),
                    quantity: o.quantity,
                    fee: 0,
                    order_id: o.id,
                    timestamp: now,
                });
            }
            !expired
        });
        // 卖单
        self.asks.retain(|o| {
            let expired = o.expire_ts.map(|ts| ts <= now).unwrap_or(false);
            if expired {
                self.balances.get_mut(&o.owner).unwrap().base += o.quantity;
                self.event_queue.push(Event {
                    event_type: EventType::Expire,
                    market: market.to_string(),
                    maker: None,
                    taker: Some(o.owner.clone()),
                    price: Some(o.price),
                    quantity: o.quantity,
                    fee: 0,
                    order_id: o.id,
                    timestamp: now,
                });
            }
            !expired
        });
    }

    /// 下单（挂入订单簿或直接撮合，支持订单有效期和自动清理过期订单）
    pub fn place_order(
        &mut self,
        market: &str,
        owner: &str,
        side: Side,
        price: u64,
        mut quantity: u64,
        now: u64,
        fee_bps: u64,
        expire_ts: Option<u64>,
    ) -> Option<u64> {
        self.clean_expired_orders(now, market);

        // 校验余额
        let bal = self.balances.entry(owner.to_string()).or_default();
        match side {
            Side::Bid => {
                let needed_quote = price * quantity;
                if bal.quote < needed_quote {
                    println!("下单失败，用户 {} 报价币余额不足", owner);
                    return None;
                }
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

        // 构造订单
        let order_id = self.next_order_id;
        self.next_order_id += 1;

        let mut order = Order {
            id: order_id,
            owner: owner.to_string(),
            side: side.clone(),
            price,
            quantity,
            expire_ts,
        };

        // 撮合逻辑
        match side {
            Side::Bid => {
                while let Some(mut best_ask) = self.asks.first().cloned() {
                    if order.price >= best_ask.price && order.quantity > 0 {
                        let deal_qty = order.quantity.min(best_ask.quantity);
                        let deal_price = best_ask.price;
                        let fee = deal_price * deal_qty * fee_bps / 10_000;
                        self.fee_receiver.collected_fee += fee;

                        // 买家获得主币，卖家获得报价币（扣除手续费）
                        self.balances.get_mut(&order.owner).unwrap().base += deal_qty;
                        self.balances.get_mut(&best_ask.owner).unwrap().quote +=
                            deal_price * deal_qty - fee;

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

                        order.quantity -= deal_qty;
                        if let Some(b0) = self.asks.first_mut() {
                            b0.quantity -= deal_qty;
                        }
                        if self.asks.first().map(|b| b.quantity == 0).unwrap_or(false) {
                            self.asks.remove(0);
                        }
                    } else {
                        break;
                    }
                }
                // 剩余未成交部分挂入订单簿
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

                        // 卖家获得报价币（扣手续费），买家获得主币
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

                        order.quantity -= deal_qty;
                        if let Some(b0) = self.bids.first_mut() {
                            b0.quantity -= deal_qty;
                        }
                        if self.bids.first().map(|b| b.quantity == 0).unwrap_or(false) {
                            self.bids.remove(0);
                        }
                    } else {
                        break;
                    }
                }
                // 剩余未成交部分挂入订单簿
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

    /// 批量撮合（对前 n 个订单尝试撮合）
    /// side: 批量撮合哪一侧（Bid/Ask）
    /// n: 前n个订单
    pub fn batch_match(&mut self, market: &str, side: Side, n: usize, now: u64, fee_bps: u64) {
        self.clean_expired_orders(now, market);
        match side {
            Side::Bid => {
                let bids = self.bids.clone();
                for order in bids.iter().take(n) {
                    self.place_order(
                        market,
                        &order.owner,
                        Side::Bid,
                        order.price,
                        order.quantity,
                        now,
                        fee_bps,
                        order.expire_ts,
                    );
                }
            }
            Side::Ask => {
                let asks = self.asks.clone();
                for order in asks.iter().take(n) {
                    self.place_order(
                        market,
                        &order.owner,
                        Side::Ask,
                        order.price,
                        order.quantity,
                        now,
                        fee_bps,
                        order.expire_ts,
                    );
                }
            }
        }
    }

    /// 批量撤销指定用户的订单
    /// ids: 要撤销的订单id列表
    pub fn batch_cancel(&mut self, market: &str, user: &str, ids: &[u64], now: u64) {
        let mut cancel_ids: Vec<u64> = ids.to_vec();
        // 买单
        self.bids.retain(|o| {
            if o.owner == user && cancel_ids.contains(&o.id) {
                let refund = o.price * o.quantity;
                self.balances.get_mut(user).unwrap().quote += refund;
                self.event_queue.push(Event {
                    event_type: EventType::Cancel,
                    market: market.to_string(),
                    maker: None,
                    taker: Some(user.to_string()),
                    price: Some(o.price),
                    quantity: o.quantity,
                    fee: 0,
                    order_id: o.id,
                    timestamp: now,
                });
                false
            } else {
                true
            }
        });
        // 卖单
        self.asks.retain(|o| {
            if o.owner == user && cancel_ids.contains(&o.id) {
                self.balances.get_mut(user).unwrap().base += o.quantity;
                self.event_queue.push(Event {
                    event_type: EventType::Cancel,
                    market: market.to_string(),
                    maker: None,
                    taker: Some(user.to_string()),
                    price: Some(o.price),
                    quantity: o.quantity,
                    fee: 0,
                    order_id: o.id,
                    timestamp: now,
                });
                false
            } else {
                true
            }
        });
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

    /// 打印平台手续费余额
    pub fn print_fee_receiver(&self) {
        println!(
            "平台累计收取手续费(报价币): {}",
            self.fee_receiver.collected_fee
        );
    }

    /// 打印事件队列
    pub fn print_events(&self) {
        println!("=== Event Queue（成交/撤单/过期历史）===");
        for event in &self.event_queue.events {
            println!("{:?}", event);
        }
    }

    /// 打印某consumer批量消费到的事件
    pub fn print_event_consume(&mut self, consumer: &str, max_events: usize) {
        let events = self.event_queue.consume_events(consumer, max_events);
        println!("=== {} 消费到的事件 ===", consumer);
        for event in events {
            println!("{:?}", event);
        }
    }
}

/// 多市场管理器
pub struct Markets {
    /// 市场状态集合（market name -> MarketState）
    pub markets: HashMap<String, MarketState>,
}

impl Markets {
    /// 新建Markets实例
    pub fn new() -> Self {
        Self {
            markets: HashMap::new(),
        }
    }

    /// 新建市场
    pub fn create_market(&mut self, market: &str) {
        self.markets
            .entry(market.to_string())
            .or_insert_with(MarketState::default);
        println!("新市场已创建: {}", market);
    }

    /// 用户充值
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
        now: u64,
        fee_bps: u64,
        expire_ts: Option<u64>,
    ) -> Option<u64> {
        if let Some(state) = self.markets.get_mut(market) {
            state.place_order(
                market, owner, side, price, quantity, now, fee_bps, expire_ts,
            )
        } else {
            println!("市场 {} 不存在", market);
            None
        }
    }

    /// 批量撮合
    pub fn batch_match(&mut self, market: &str, side: Side, n: usize, now: u64, fee_bps: u64) {
        if let Some(state) = self.markets.get_mut(market) {
            state.batch_match(market, side, n, now, fee_bps);
        }
    }

    /// 批量撤销
    pub fn batch_cancel(&mut self, market: &str, user: &str, ids: &[u64], now: u64) {
        if let Some(state) = self.markets.get_mut(market) {
            state.batch_cancel(market, user, ids, now);
        }
    }

    /// 打印市场订单簿
    pub fn print_market_book(&self, market: &str) {
        if let Some(state) = self.markets.get(market) {
            println!("=== 市场 {} 订单簿 ===", market);
            state.print_book();
        } else {
            println!("市场 {} 不存在", market);
        }
    }

    /// 打印市场余额
    pub fn print_market_balances(&self, market: &str) {
        if let Some(state) = self.markets.get(market) {
            println!("=== 市场 {} 用户余额 ===", market);
            state.print_balances();
        } else {
            println!("市场 {} 不存在", market);
        }
    }

    /// 打印市场手续费池
    pub fn print_market_fee_receiver(&self, market: &str) {
        if let Some(state) = self.markets.get(market) {
            println!("=== 市场 {} 平台手续费 ===", market);
            state.print_fee_receiver();
        }
    }

    /// 打印市场事件队列
    pub fn print_market_events(&self, market: &str) {
        if let Some(state) = self.markets.get(market) {
            println!("=== 市场 {} Event Queue ===", market);
            state.print_events();
        }
    }

    /// 打印市场中某consumer批量消费到的事件
    pub fn print_market_event_consume(&mut self, market: &str, consumer: &str, max_events: usize) {
        if let Some(state) = self.markets.get_mut(market) {
            state.print_event_consume(consumer, max_events);
        }
    }
}

fn main() {
    let mut markets = Markets::new();
    let fee_bps = 30; // 0.3%，手续费率
    let mut now = 1_000_000_000u64; // 时间戳

    // 新建市场
    markets.create_market("SOL/USDC");

    // 用户充值
    markets.deposit("SOL/USDC", "Alice", 100, 2000);
    markets.deposit("SOL/USDC", "Bob", 50, 1000);

    // Alice下买单，有效期5秒
    let alice_bid = markets.place_order(
        "SOL/USDC",
        "Alice",
        Side::Bid,
        10,
        10,
        now,
        fee_bps,
        Some(now + 5),
    );
    now += 1;
    // Bob下卖单，有效期10秒
    let bob_ask = markets.place_order(
        "SOL/USDC",
        "Bob",
        Side::Ask,
        10,
        5,
        now,
        fee_bps,
        Some(now + 10),
    );

    now += 6;
    // 超过Alice订单有效期，自动过期
    println!("\n--- 撮合前自动清理过期订单 ---");
    markets.print_market_book("SOL/USDC");
    markets.print_market_events("SOL/USDC");

    // 再来一笔买单并撮合
    let _ = markets.place_order(
        "SOL/USDC",
        "Alice",
        Side::Bid,
        10,
        10,
        now,
        fee_bps,
        Some(now + 10),
    );
    now += 1;

    // 批量撮合（批量对Bid订单撮合2笔）
    println!("\n--- 批量撮合 ---");
    markets.batch_match("SOL/USDC", Side::Bid, 2, now, fee_bps);

    // 批量撤单（批量撤销Bob所有挂单）
    println!("\n--- 批量撤单 ---");
    if let Some(state) = markets.markets.get("SOL/USDC") {
        let bob_orders: Vec<u64> = state
            .asks
            .iter()
            .filter(|o| o.owner == "Bob")
            .map(|o| o.id)
            .collect();
        markets.batch_cancel("SOL/USDC", "Bob", &bob_orders, now);
    }

    // 查看订单簿、余额、手续费、事件队列
    markets.print_market_book("SOL/USDC");
    markets.print_market_balances("SOL/USDC");
    markets.print_market_fee_receiver("SOL/USDC");
    markets.print_market_events("SOL/USDC");

    // 事件队列批量消费（模拟前端或cranker消费事件）
    println!("\n--- crank1 首次批量消费事件 ---");
    markets.print_market_event_consume("SOL/USDC", "crank1", 10);

    println!("\n--- crank1 再次消费（无新事件） ---");
    markets.print_market_event_consume("SOL/USDC", "crank1", 10);

    println!("\n--- crank2 首次批量消费事件（独立消费指针） ---");
    markets.print_market_event_consume("SOL/USDC", "crank2", 3);
}
