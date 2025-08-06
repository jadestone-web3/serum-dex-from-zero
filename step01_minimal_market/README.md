## What is the core function of Serum DEX?

The core essence of Serum DEX is ** an on chain order book matching engine ** used to match buyers and sellers.

If only the smallest skeleton is retained, the most essential function is:
- User hangs buy/sell orders (placing orders)
- System matching orders (matching engine)
- Asset transfer (settlement) after matching

We can use only the memory structure without considering account security for now PDA、 All the complex designs such as permissions, transaction fees, event queues, etc., are implemented using the simplest Rust code to accomplish the task of "placing and matching orders in a market".

