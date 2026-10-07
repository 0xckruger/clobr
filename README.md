# clobr

A limit order book and matching engine in Rust.

## Design goals
clobr (central limit order book, rust) is a price-time priority order book allowing for market/limit orders to be matched either in full 
or partially according to price level and volume. Partial market orders will have the remaining volume cancelled.
Cancel orders will be supported but cancellation of fully filled orders will be rejected; partially filled orders may have
the remaining volume cancelled.

Efforts will be taken to reduce latency:
- The matching engine will be single threaded
- Orders are handled in sequence through a queue
- Only integer prices supported by formatting prices in cents; i.e. 7550 == 75.50, 83769 == 837.69
- No allocations will be made on the hot path (matching of orders)

Out of scope for this project will be:
- Advanced types of orders (stop loss, stop limit, trailing stop, hidden, iceberg etc.)
- Advanced order/execution history storage (will be log-based)
- Accounts/authorization
- Fractional shares (e.g. 0.5 AAPL)
- Self-trade prevention

Current goal is to build core matching engine before adding gateway with REST/HTTP/WS at a later phase.

## Book structure
Book will be represented by 2x `BTreeMap<Price, Level>`. One for bids and one for asks.
A BTreeMap naturally lends itself to maintaining an O(log N) access time per price level.
By constructing a `BTreeMap<Price, Level>` we can walk the (bid|ask) tree if we have a (sell|buy) order, beginning at
the most competitive price and conuming volume at each level until either no volume at the requested price (or any price,
if a market order) remain (in which case it would become a resting limit order) or the order is fulfilled.


```      
                     root
        [100          |           105]
          /                         \
         /                           \
    [ 95 | 98 ]                  [102 | 108]
    /     |    \                 /    |     \
  [90]   [95]  [98]          [102]   [105]  [110]
 Level   Level  Level        Level   Level   Level
```

## Build

```sh
cargo test --workspace
cargo bench -p clobr
```
