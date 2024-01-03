// use uuid::Uuid;

use common::{Order, OrderType, VisibleOrder};
use serde::{Serialize, Deserialize};

// TODO handle true equality
// TODO hidden orders back of queue
// TODO seperate bid compare function that preserves time ordering
// TODO make sure that hidden orders still have time ordering in queue

fn compare_ask_orders(a: &Order, b: &Order) -> std::cmp::Ordering {
	if a.price > b.price {
		std::cmp::Ordering::Greater
	} else if a.price < b.price {
		std::cmp::Ordering::Less
	} else { match (a.hidden, b.hidden) {
		(true, false) => std::cmp::Ordering::Greater,
		(false, true) => std::cmp::Ordering::Less,
		_ => {
			if a.time > b.time {
				std::cmp::Ordering::Greater
			} else if a.time < b.time { std::cmp::Ordering::Less }
			else { std::cmp::Ordering::Equal }
		}
	}}
}

fn compare_bid_orders(a: &Order, b: &Order) -> std::cmp::Ordering {
	if a.price > b.price {
		std::cmp::Ordering::Less
	} else if a.price < b.price {
		std::cmp::Ordering::Greater
	} else { match (a.hidden, b.hidden) {
		(true, false) => std::cmp::Ordering::Greater,
		(false, true) => std::cmp::Ordering::Less,
		_ => {
			if a.time > b.time {
				std::cmp::Ordering::Greater
			} else if a.time < b.time { std::cmp::Ordering::Less }
			else { std::cmp::Ordering::Equal }
		}
	}}
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Book {
	pub buy: Vec<Order>,
	pub sell: Vec<Order>,
	pub matches: Vec<common::Match>,
}

impl Book {
	pub fn new() -> Self {
		Self {
			buy: Vec::new(),
			sell: Vec::new(),
			matches: Vec::new(),
		}
	}

	pub fn add_order(&mut self, order: Order) {
		let (queue, other): (&mut Vec<Order>, &mut Vec<Order>) =
			if order.otype == OrderType::Ask {
				(&mut self.sell, &mut self.buy)
			} else { (&mut self.buy, &mut self.sell) };

		queue.push(order.clone());
		if order.otype == OrderType::Bid {		
			queue.sort_by(compare_bid_orders);			
		} else { queue.sort_by(compare_ask_orders); }

		let mut offset = 0;
		for (i, o) in other.clone().iter().enumerate() {
			if o.price < order.price && order.otype == OrderType::Ask { continue }
			if o.price > order.price && order.otype == OrderType::Bid { break }
			self.matches.push(common::Match {
				buyer: if o.otype == OrderType::Bid { o.trader } else { order.trader },
				seller: if o.otype == OrderType::Ask { o.trader } else { order.trader },
				price: if order.time < o.time { order.price } else { o.price },
				qty: o.qty,				
			});			
			if o.qty < queue[0].qty {
				other.remove(i-offset);
				queue[0].qty -= o.qty;
				offset += 1;
			} else if o.qty == queue[0].qty {
				other.remove(i-offset);
				queue.remove(0);
				break;
			} else {
				self.matches.last_mut().unwrap().qty = queue[0].qty;
				other[i-offset].qty -= queue[0].qty;				
				queue.remove(0);
				break;
			}
		}
	}

	pub fn drop_order(&mut self, id: i64) {
		for (i, o) in self.buy.iter().enumerate() {
			if o.id == id {
				// let ret = o.trader;
				self.buy.remove(i);
				return;
				// return ret;
			}
		}
		for (i, o) in self.sell.iter().enumerate() {
			if o.id == id {
				// let ret = o.trader;
				self.sell.remove(i);
				return;
				// return ret;
			}
		}
		todo!()
	}
}
