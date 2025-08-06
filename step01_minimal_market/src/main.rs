mod openbook;

use openbook::{Order, OrderBook, Side};

fn main() {
    println!("=== 最小化订单簿演示 ===");

    let mut book = OrderBook::new();

    // 用户A挂买单
    book.place_order(Order {
        owner: "A".to_string(),
        side: Side::Bid,
        price: 10,
        quantity: 5,
    });

    // 用户B挂卖单
    book.place_order(Order {
        owner: "B".to_string(),
        side: Side::Ask,
        price: 11,
        quantity: 2,
    });

    // 用户C挂卖单，价格10，触发撮合
    book.place_order(Order {
        owner: "C".to_string(),
        side: Side::Ask,
        price: 10,
        quantity: 3,
    });

    book.print_book();
}
