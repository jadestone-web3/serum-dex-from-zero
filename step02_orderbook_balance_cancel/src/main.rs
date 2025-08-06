mod openbook;

use openbook::{Order, OrderBook, Side};

fn main() {
    let mut book = OrderBook::new();

    // 模拟链上充值：用户A、B
    book.deposit("A", 100, 2000);
    book.deposit("B", 50, 1000);

    // 用户A挂买单（价格10，数量10）
    let a_bid_id = book.place_order("A", Side::Bid, 10, 10);

    // 用户B挂卖单（价格10，数量5）
    let b_ask_id = book.place_order("B", Side::Ask, 10, 5);

    // 用户A撤销自己的买单（如果有剩余）
    if let Some(id) = a_bid_id {
        book.cancel_order("A", id);
    }

    book.print_book();
    book.print_balances();
}
