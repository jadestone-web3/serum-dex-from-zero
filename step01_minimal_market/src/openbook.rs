#[derive(Debug, Clone)]
pub enum Side {
    Bid, // 买单
    Ask, // 卖单
}

#[derive(Debug, Clone)]
pub struct Order {
    pub owner: String,
    pub side: Side,
    pub price: u64,    // 价格
    pub quantity: u64, // 数量
}

pub struct OrderBook {
    bids: Vec<Order>, // 买单簿（价格降序）
    asks: Vec<Order>, // 卖单簿（价格升序）
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            bids: vec![],
            asks: vec![],
        }
    }

    // 下单：存到订单簿（并尝试撮合）
    pub fn place_order(&mut self, mut order: Order) {
        match order.side {
            Side::Bid => {
                // 买单：尝试和最低的卖单撮合
                while let Some(mut best_ask) = self.asks.first().cloned() {
                    if order.price >= best_ask.price && order.quantity > 0 {
                        // 撮合成交
                        let qty = order.quantity.min(best_ask.quantity);
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
                // 如果有剩余未成交，插入买单簿
                if order.quantity > 0 {
                    self.bids.push(order);
                    self.bids.sort_by(|a, b| b.price.cmp(&a.price)); // 价格降序
                }
            }
            Side::Ask => {
                // 卖单：尝试和最高的买单撮合
                while let Some(mut best_bid) = self.bids.first().cloned() {
                    if order.price <= best_bid.price && order.quantity > 0 {
                        let qty = order.quantity.min(best_bid.quantity);
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
                    self.asks.push(order);
                    self.asks.sort_by(|a, b| a.price.cmp(&b.price)); // 价格升序
                }
            }
        }
    }

    pub fn print_book(&self) {
        println!("买单簿: {:?}", self.bids);
        println!("卖单簿: {:?}", self.asks);
    }

    // 测试用的getter方法
    pub fn get_bids(&self) -> &[Order] {
        &self.bids
    }

    pub fn get_asks(&self) -> &[Order] {
        &self.asks
    }

    pub fn get_bids_count(&self) -> usize {
        self.bids.len()
    }

    pub fn get_asks_count(&self) -> usize {
        self.asks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_book_creation() {
        let book = OrderBook::new();
        assert!(book.bids.is_empty());
        assert!(book.asks.is_empty());
    }

    #[test]
    fn test_place_bid_order() {
        let mut book = OrderBook::new();
        let order = Order {
            owner: "A".to_string(),
            side: Side::Bid,
            price: 10,
            quantity: 5,
        };

        book.place_order(order);
        assert_eq!(book.bids.len(), 1);
        assert_eq!(book.asks.len(), 0);
        assert_eq!(book.bids[0].owner, "A");
        assert_eq!(book.bids[0].price, 10);
        assert_eq!(book.bids[0].quantity, 5);
    }

    #[test]
    fn test_place_ask_order() {
        let mut book = OrderBook::new();
        let order = Order {
            owner: "B".to_string(),
            side: Side::Ask,
            price: 11,
            quantity: 3,
        };

        book.place_order(order);
        assert_eq!(book.bids.len(), 0);
        assert_eq!(book.asks.len(), 1);
        assert_eq!(book.asks[0].owner, "B");
        assert_eq!(book.asks[0].price, 11);
        assert_eq!(book.asks[0].quantity, 3);
    }

    #[test]
    fn test_matching_bid_ask() {
        let mut book = OrderBook::new();

        // 先挂买单
        book.place_order(Order {
            owner: "A".to_string(),
            side: Side::Bid,
            price: 10,
            quantity: 5,
        });

        // 再挂卖单，应该撮合成交
        book.place_order(Order {
            owner: "B".to_string(),
            side: Side::Ask,
            price: 10,
            quantity: 3,
        });

        // 买单应该剩余2个
        assert_eq!(book.bids.len(), 1);
        assert_eq!(book.bids[0].quantity, 2);
        // 卖单应该完全成交
        assert_eq!(book.asks.len(), 0);
    }

    #[test]
    fn test_price_priority() {
        let mut book = OrderBook::new();

        // 挂多个买单，价格不同
        book.place_order(Order {
            owner: "A".to_string(),
            side: Side::Bid,
            price: 10,
            quantity: 5,
        });

        book.place_order(Order {
            owner: "B".to_string(),
            side: Side::Bid,
            price: 11,
            quantity: 3,
        });

        // 价格高的应该在前面
        assert_eq!(book.bids[0].price, 11);
        assert_eq!(book.bids[1].price, 10);
    }

    #[test]
    fn test_partial_fill() {
        let mut book = OrderBook::new();

        // 挂买单
        book.place_order(Order {
            owner: "A".to_string(),
            side: Side::Bid,
            price: 10,
            quantity: 5,
        });

        // 挂卖单，数量大于买单
        book.place_order(Order {
            owner: "B".to_string(),
            side: Side::Ask,
            price: 10,
            quantity: 8,
        });

        // 买单应该完全成交
        assert_eq!(book.bids.len(), 0);
        // 卖单应该剩余3个
        assert_eq!(book.asks.len(), 1);
        assert_eq!(book.asks[0].quantity, 3);
    }
}
