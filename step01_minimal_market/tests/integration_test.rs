use step01_minimal_market::openbook::{Order, OrderBook, Side};

#[test]
fn test_complete_trading_scenario() {
    let mut book = OrderBook::new();

    // 场景1：多个用户下单，测试撮合逻辑
    book.place_order(Order {
        owner: "Alice".to_string(),
        side: Side::Bid,
        price: 100,
        quantity: 10,
    });

    book.place_order(Order {
        owner: "Bob".to_string(),
        side: Side::Ask,
        price: 105,
        quantity: 5,
    });

    book.place_order(Order {
        owner: "Charlie".to_string(),
        side: Side::Bid,
        price: 102,
        quantity: 8,
    });

    // 验证订单簿状态
    assert_eq!(book.get_bids_count(), 2);
    assert_eq!(book.get_asks_count(), 1);

    // 验证价格排序
    let bids = book.get_bids();
    let asks = book.get_asks();
    assert_eq!(bids[0].price, 102); // 价格高的买单在前
    assert_eq!(bids[1].price, 100);
    assert_eq!(asks[0].price, 105);

    // 场景2：触发撮合
    book.place_order(Order {
        owner: "David".to_string(),
        side: Side::Ask,
        price: 101,
        quantity: 15,
    });

    // 验证撮合结果
    // David的卖单应该与Charlie的买单部分撮合
    // Charlie的买单应该完全成交（8个）
    // David的卖单应该剩余7个
    assert_eq!(book.get_bids_count(), 1); // 只剩下Alice的买单
    let bids = book.get_bids();
    assert_eq!(bids[0].owner, "Alice");
    assert_eq!(bids[0].quantity, 10);

    assert_eq!(book.get_asks_count(), 2); // David的剩余卖单 + Bob的卖单
}

#[test]
fn test_edge_cases() {
    let mut book = OrderBook::new();

    // 测试边界情况：相同价格
    book.place_order(Order {
        owner: "A".to_string(),
        side: Side::Bid,
        price: 100,
        quantity: 5,
    });

    book.place_order(Order {
        owner: "B".to_string(),
        side: Side::Bid,
        price: 100,
        quantity: 3,
    });

    // 相同价格的买单应该按时间顺序排列
    assert_eq!(book.get_bids_count(), 2);
    let bids = book.get_bids();
    assert_eq!(bids[0].owner, "A");
    assert_eq!(bids[1].owner, "B");

    // 测试零数量订单
    book.place_order(Order {
        owner: "C".to_string(),
        side: Side::Ask,
        price: 100,
        quantity: 0,
    });

    // 零数量订单不应该被添加到订单簿
    assert_eq!(book.get_asks_count(), 0);
}

#[test]
fn test_market_order_simulation() {
    let mut book = OrderBook::new();

    // 先添加一些卖单
    book.place_order(Order {
        owner: "Seller1".to_string(),
        side: Side::Ask,
        price: 100,
        quantity: 5,
    });

    book.place_order(Order {
        owner: "Seller2".to_string(),
        side: Side::Ask,
        price: 101,
        quantity: 5,
    });

    // 模拟市价单：以最高价格买入
    book.place_order(Order {
        owner: "Market_Buyer".to_string(),
        side: Side::Bid,
        price: u64::MAX, // 最高价格
        quantity: 10,
    });

    // 市价买单应该与最低价格的卖单撮合
    // 应该先与Seller1的100价格卖单撮合5个，再与Seller2的101价格卖单撮合5个
    // 完全成交的订单不会被添加到订单簿
    assert_eq!(book.get_bids_count(), 0);

    // 所有卖单都应该被完全撮合
    assert_eq!(book.get_asks_count(), 0);
}
