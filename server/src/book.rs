// use uuid::Uuid;

#[derive(PartialEq, Clone, Debug)]
pub enum OrderType {
    Ask,
    Bid
}

#[derive(Debug, Clone)]
pub struct Order {
    pub otype: OrderType,
    pub price: f64,
    pub trader: i64,
    pub qty: u64,
    pub time: u128,
    pub id: i64,
	pub hidden: bool,
}

// TODO handle equality
fn compare_orders(a: &Order, b: &Order) -> std::cmp::Ordering {
    if a.price == b.price {
        return if a.time > b.time {
            std::cmp::Ordering::Greater
        } else { std::cmp::Ordering::Less }
    }
	if a.price > b.price {
        std::cmp::Ordering::Greater
    } else { std::cmp::Ordering::Less }
}

pub struct Book {
    pub buy: Vec<Order>,
    pub sell: Vec<Order>,
}

impl Book {
    pub fn new() -> Self {
        Self {
            buy: Vec::new(),
            sell: Vec::new()
        }
    }

    pub fn add_order(&mut self, order: Order) {		
		let (queue, other): (&mut Vec<Order>, &mut Vec<Order>) =
			if order.otype == OrderType::Ask {
				(&mut self.sell, &mut self.buy)
			} else { (&mut self.buy, &mut self.sell) };
		
		queue.push(order.clone());
		queue.sort_by(compare_orders);

		for (i, o) in other.clone().iter().enumerate() {
			if o.price < order.price && order.otype == OrderType::Ask { break }
			if o.price > order.price && order.otype == OrderType::Bid { break }
			if o.qty < order.qty {
				other.remove(i);
				queue[0].qty -= o.qty;
			} else if o.qty == order.qty {
				other.remove(i);
				queue.remove(0);
				break;
			} else {
				queue.remove(0);
				other[i].qty -= order.qty;
				break;
			}
		}
    }	
}
