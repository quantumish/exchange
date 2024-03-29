use serde::{Serialize, Deserialize};

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum OrderType {
	Bid,
	Ask
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub otype: OrderType,
    pub price: f64,
    pub trader: u64,
    pub qty: u64,
    pub time: u128,
    pub id: i64,
	pub hidden: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderReq {
	pub kind: OrderType,
	pub qty: u64,
	pub price: f64,
	pub hidden: bool
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisibleOrder {
	pub qty: u64,
	pub price: f64,
	pub mine: bool
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Match {
	pub buyer: u64,
	pub seller: u64,
	pub qty: u64,
	pub price: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RequestBody {
	Cancel(i64),
	ExchangeOrder(OrderReq),
	DarkpoolOrder(OrderReq),
	Get,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
	pub tid: u64,
	pub body: RequestBody,
}

impl Request {
	pub fn new(tid: u64, body: RequestBody) -> Self {
		Request { tid, body }
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraderStatus {
	pub goal: OrderType,
	pub amount: u64,
	pub tolerance: u64,
	pub orders: u64,
	pub done: u64,
	pub opp: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Response {
	pub book: (Vec<VisibleOrder>, Vec<VisibleOrder>),
	pub matches: Vec<VisibleOrder>,
	pub orders: Vec<Order>,
	pub dark_matches: Vec<VisibleOrder>,
	pub dark_orders: Vec<Order>,
	pub status: TraderStatus
}

// #[derive(Clone, Serialize, Deserialize)]
// pub struct DarkpoolResponse {
// 	pub matches: Vec<VisibleOrder>,
// 	pub orders: Vec<Order>
// }

// #[derive(Clone, Serialize, Deserialize)]
// enum Response {
// 	Exchange(ExchangeResponse),
// 	Darkpool(DarkpoolResponse),
// }
