use std::collections::HashMap;

// 订单方向：买单 or 卖单
#[derive(Debug, Clone)]
pub enum Side {
    Bid, // 买单
    Ask, // 卖单
}

// 订单结构，带有唯一id
#[derive(Debug, Clone)]
pub struct Order {
    pub id: u64,       // 订单ID（唯一）
    pub owner: String, // 挂单用户
    pub side: Side,    // 买 or 卖
    pub price: u64,    // 价格
    pub quantity: u64, // 剩余数量
}

// 用户余额，分别为主币与报价币
#[derive(Debug, Default)]
pub struct UserBalance {
    pub base: u64,  // 主币余额（如SOL）
    pub quote: u64, // 报价币余额（如USDC）
}

// 订单簿结构
pub struct OrderBook {
    next_order_id: u64,                     // 自增订单ID
    bids: Vec<Order>,                       // 买单簿（价格降序）
    asks: Vec<Order>,                       // 卖单簿（价格升序）
    balances: HashMap<String, UserBalance>, // 用户余额（实际链上应为账户结构，这里仅用于模拟）
}

/*
实际链上开发说明：
- 用户余额应由链上账户结构管理，余额变更需通过链上指令与账户权限校验；
- 撤单、撮合、下单等都会影响链上账户和Token账户余额；
- 这里用HashMap仅做本地模拟，方便理解流程，实际部署应严格依赖区块链账户模型。
*/

impl OrderBook {
    pub fn new() -> Self {
        Self {
            next_order_id: 1,
            bids: vec![],
            asks: vec![],
            balances: HashMap::new(),
        }
    }

    // 用户充值（模拟现实中链上转账到合约或账户）
    pub fn deposit(&mut self, user: &str, base: u64, quote: u64) {
        let bal = self.balances.entry(user.to_string()).or_default();
        bal.base += base;
        bal.quote += quote;
        println!("用户 {} 充值：主币 {}，报价币 {}", user, base, quote);
    }

    // 下单：校验余额 -> 撮合 -> 未成交部分入订单簿 -> 冻结余额
    pub fn place_order(
        &mut self,
        owner: &str,
        side: Side,
        price: u64,
        quantity: u64,
    ) -> Option<u64> {
        let mut quantity = quantity;
        // 1. 校验余额
        let bal = self.balances.entry(owner.to_string()).or_default();
        match side {
            Side::Bid => {
                // 买单：需要冻结报价币
                let needed_quote = price * quantity;
                if bal.quote < needed_quote {
                    println!("下单失败，用户 {} 报价币余额不足", owner);
                    return None;
                }
                bal.quote -= needed_quote; // 先全部冻结，未成交部分后返还
            }
            Side::Ask => {
                // 卖单：需要冻结主币
                if bal.base < quantity {
                    println!("下单失败，用户 {} 主币余额不足", owner);
                    return None;
                }
                bal.base -= quantity;
            }
        }

        // 2. 创建订单
        let order_id = self.next_order_id;
        self.next_order_id += 1;
        let mut order = Order {
            id: order_id,
            owner: owner.to_string(),
            side: side.clone(),
            price,
            quantity,
        };

        // 3. 尝试撮合
        match side {
            Side::Bid => {
                while let Some(mut best_ask) = self.asks.first().cloned() {
                    if order.price >= best_ask.price && order.quantity > 0 {
                        let qty = order.quantity.min(best_ask.quantity);
                        // 结算：买家付报价币，卖家得报价币；卖家付主币，买家得主币
                        self.balances.get_mut(&order.owner).unwrap().base += qty;
                        self.balances.get_mut(&best_ask.owner).unwrap().quote +=
                            best_ask.price * qty;
                        self.balances.get_mut(&best_ask.owner).unwrap().base += 0; // 这里可以扣减已锁定主币，但已在挂单时扣除了

                        println!(
                            "撮合成交: 买家:{} 卖家:{} 价:{} 数量:{}",
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
                    // 未成交部分，返还部分报价币
                    let refund = (order.price * order.quantity) as u64;
                    self.balances.get_mut(&order.owner).unwrap().quote += refund;
                    // 挂入订单簿
                    self.bids.push(order.clone());
                    self.bids.sort_by(|a, b| b.price.cmp(&a.price));
                    println!(
                        "买单部分未成交，剩余数量 {} 进入订单簿，订单ID={}",
                        order.quantity, order.id
                    );
                }
            }
            Side::Ask => {
                while let Some(mut best_bid) = self.bids.first().cloned() {
                    if order.price <= best_bid.price && order.quantity > 0 {
                        let qty = order.quantity.min(best_bid.quantity);
                        // 结算：卖家得报价币，买家得主币
                        self.balances.get_mut(&order.owner).unwrap().quote += best_bid.price * qty;
                        self.balances.get_mut(&best_bid.owner).unwrap().base += qty;

                        println!(
                            "撮合成交: 卖家:{} 买家:{} 价:{} 数量:{}",
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
                    // 未成交部分，返还主币
                    self.balances.get_mut(&order.owner).unwrap().base += order.quantity;
                    // 挂入订单簿
                    self.asks.push(order.clone());
                    self.asks.sort_by(|a, b| a.price.cmp(&b.price));
                    println!(
                        "卖单部分未成交，剩余数量 {} 进入订单簿，订单ID={}",
                        order.quantity, order.id
                    );
                }
            }
        }
        Some(order_id)
    }

    // 撤单：指定订单ID撤销挂单
    pub fn cancel_order(&mut self, user: &str, order_id: u64) -> bool {
        // 买单
        if let Some(pos) = self
            .bids
            .iter()
            .position(|o| o.id == order_id && o.owner == user)
        {
            let order = self.bids.remove(pos);
            // 返还未成交部分的报价币
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
            // 返还未成交部分的主币
            self.balances.get_mut(user).unwrap().base += order.quantity;
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
            println!(
                "用户 {} 主币余额:{} 报价币余额:{}",
                user, bal.base, bal.quote
            );
        }
    }
}
