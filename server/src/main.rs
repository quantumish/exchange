mod book;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use std::path::Path;

use book::*;

// main todos
// TODO add goals for each person
   // TODO limiting num orders + general reasonable guards
   // TODO track how near target price?
   // TODO VWAP
// TODO auction darkpool
// TODO more order types: reserve, midpoint, instacancel

use common::*;
/// Currently a modified version of `poem`'s default websocket-chat example
use futures_util::{SinkExt, StreamExt};
use poem::{
    get, handler,
    listener::TcpListener,
    web::{websocket::{Message, WebSocket}, Data},
    Route, Server, IntoResponse, EndpointExt
};
use rand::{thread_rng, Rng};


const MAX_ORDERS: u64 = 200;
const MAX_ORDER_SIZE: u64 = 50_000;
const TOTAL_SHARES: u64 = 1_000_000;

static LOB: std::sync::Mutex<Option<Book>> = std::sync::Mutex::new(None);
static DARKPOOL: std::sync::Mutex<Option<Book>> = std::sync::Mutex::new(None);
static PREV: std::sync::Mutex<OrderType> = std::sync::Mutex::new(OrderType::Ask);


lazy_static::lazy_static! {
	static ref USERS: std::sync::Mutex<HashMap<u64, TraderStatus>>
		= std::sync::Mutex::new(HashMap::new());
}

pub fn gen_id() -> i64 {
    static STATE: std::sync::Mutex<Option<rustflake::Snowflake>> = std::sync::Mutex::new(None);

    STATE
        .lock()
        .unwrap()
        .get_or_insert_with(|| rustflake::Snowflake::new(1_564_790_400_000, 2, 1))
        .generate()
}

fn get_book(path: &str) -> Book {
	if Path::new(path).exists() {
		serde_json::from_str(
			&std::fs::read_to_string("./book.json").unwrap()
		).unwrap()
	} else {
		Book::new()
	}	
}

#[handler]
fn ws(
    ws: WebSocket,
	addr: &poem::web::RemoteAddr,
    sender: Data<&tokio::sync::broadcast::Sender<(Book, Book)>>,
) -> impl IntoResponse {
    let sender = sender.clone(); // Subscribe to global channel	
    ws.on_upgrade(move |socket| async move {		
        let mut receiver = sender.subscribe();
        let (mut sink, mut stream) = socket.split();

		let mut tid: u64 = u64::MAX;
		
        tokio::spawn(async move {
            // Wait to receive a message from person who opened websocket
            while let Some(Ok(mesg)) = stream.next().await { 
                if let Message::Binary(msg) = mesg {
					let duration_since_epoch = std::time::SystemTime::now()
						.duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap();
					let req: common::Request = rmp_serde::from_slice(&msg).unwrap();
					tid = req.tid;

					let mut users = USERS.lock().unwrap();
					let existing = *PREV.lock().unwrap();
					if users.get(&tid).is_none() {
						users.insert(tid, TraderStatus {
							goal: if existing == OrderType::Ask {
								*PREV.lock().unwrap() = OrderType::Bid;
								OrderType::Bid
							} else {
								*PREV.lock().unwrap() = OrderType::Ask;
								OrderType::Ask
							},				
							amount: TOTAL_SHARES,
							tolerance: TOTAL_SHARES/10,
							orders: 0,
							done: 0,
							opp: 0
						});
					}
					
					let mut book = LOB.lock().unwrap();
					let mut tmp = book.get_or_insert_with(|| get_book("./book.json"));
					let mut pool = DARKPOOL.lock().unwrap();
					let mut dtmp = pool.get_or_insert_with(|| get_book("./dpool.json"));
					let dict = USERS.lock().unwrap().clone();
					let user = dict.get(&tid).unwrap();
					match req.body {
						common::RequestBody::ExchangeOrder(o) => {
							if (o.kind != user.goal && user.opp+o.qty > user.tolerance) ||
								(o.kind == user.goal && user.done >= user.amount) ||
								(user.orders >= MAX_ORDERS) || (o.price <= 0.01) ||
								(o.qty > MAX_ORDER_SIZE) 
							{
								continue;
							}
							// FIXME DO NOTL EAVE ME I NHERE AAAA
							if o.qty == 15 { panic!() }
							tmp.add_order(Order {
								otype: o.kind,
								price: o.price,
								trader: tid,
								qty: o.qty,
								time: duration_since_epoch.as_nanos(),
								id: gen_id(),
								hidden: o.hidden,
							});							
						},
						common::RequestBody::DarkpoolOrder(o) => {
							if (o.kind != user.goal && user.opp+o.qty > user.tolerance)  ||
								(o.kind == user.goal && user.done >= user.amount) ||
								(user.orders >= MAX_ORDERS) || (o.price <= 0.01) ||
								(o.qty > MAX_ORDER_SIZE)
							{
								continue;
							}
							dtmp.add_order(Order {
								otype: o.kind,
								price: o.price,
								trader: tid,
								qty: o.qty,
								time: duration_since_epoch.as_nanos(),
								id: gen_id(),
								hidden: o.hidden,
							});
						}
						common::RequestBody::Get => (),
						common::RequestBody::Cancel(id) => {							
							tmp.drop_order(id); // HACK HACK HACK
						},
					}

					std::fs::write("./book.json", serde_json::to_string(&tmp.clone()).unwrap()).unwrap();
					std::fs::write("./dpool.json", serde_json::to_string(&dtmp.clone()).unwrap()).unwrap();
					std::fs::write("./users.json", serde_json::to_string(&dict).unwrap()).unwrap();

					let (other, other2) = (tmp.clone(), dtmp.clone());
					let mut all_matches = Vec::new();
					all_matches.extend(other.matches.iter());
					all_matches.extend(other2.matches.iter());
					let mut dict = USERS.lock().unwrap();
					for tid in dict.clone().keys() {
						let user = dict.get_mut(&tid).unwrap();
						user.opp = 0;
						user.done = 0;
						user.orders = 0;
						for o in all_matches.clone() {
							if o.seller == *tid || o.buyer == *tid {
								user.orders += 1;
							} else { continue }
							if o.seller == *tid && o.buyer == *tid {
								continue
							} else if (o.seller == *tid && OrderType::Ask == user.goal) ||
								(o.buyer == *tid && OrderType::Bid == user.goal) {
								user.done += o.qty;
							} else {
								user.opp += o.qty;
							}
						}
						let mut their_orders = Vec::new();
						their_orders
							.extend(tmp.buy.iter().filter(|o| o.trader == *tid).map(|o| o.clone()));
						their_orders
							.extend(tmp.sell.iter().filter(|o| o.trader == *tid).map(|o| o.clone()));
						let mut their_dark_orders = Vec::new();
						their_dark_orders
							.extend(dtmp.buy.iter().filter(|o| o.trader == *tid).map(|o| o.clone()));
						their_dark_orders
							.extend(dtmp.sell.iter().filter(|o| o.trader == *tid).map(|o| o.clone()));
						if user.opp >= user.tolerance {							
							for o in their_orders.clone() {
								if o.otype != user.goal { tmp.drop_order(o.id); }
							}
							for o in their_dark_orders.clone() {
								if o.otype != user.goal { dtmp.drop_order(o.id); }
							}
						}
						if user.done >= user.amount {
							for o in their_orders.clone() { tmp.drop_order(o.id);}
							for o in their_dark_orders.clone() { dtmp.drop_order(o.id); }
						}
					}

					
					// Send to global channel
                    if sender.send((tmp.clone(), dtmp.clone())).is_err() {
                        break;
                    }
                }
            }
        });

        tokio::spawn(async move {
            // Wait to receive a message from global channel
            while let Ok(msg) = receiver.recv().await {
				let buys: Vec<common::VisibleOrder> = msg.0.buy.iter().filter(|o| !o.hidden)
					.map(|o| {
						common::VisibleOrder { qty: o.qty, price: o.price, mine: o.trader == tid }
					}).collect();
				let sells: Vec<common::VisibleOrder> = msg.0.sell.iter().filter(|o| !o.hidden)
					.map(|o| {
						common::VisibleOrder { qty: o.qty, price: o.price, mine: o.trader == tid }
					}).collect();

				
				let mut their_orders = Vec::new();
				their_orders.extend(msg.0.buy.iter().filter(|o| { o.trader == tid } ).map(|o| o.clone()));
				their_orders.extend(msg.0.sell.iter().filter(|o| o.trader == tid).map(|o| o.clone()));

				let mut their_dark_orders = Vec::new();
				their_dark_orders
					.extend(msg.1.buy.iter().filter(|o| { o.trader == tid } ).map(|o| o.clone()));
				their_dark_orders
					.extend(msg.1.sell.iter().filter(|o| o.trader == tid).map(|o| o.clone()));				

				let mut all_matches = Vec::new();
				let cloned = msg.clone();
				all_matches.extend(cloned.0.matches.iter());
				all_matches.extend(cloned.1.matches.iter());
				let mesg = { 
				let mut dict = USERS.lock().unwrap();
				let user = dict.get(&tid).unwrap();
					
				rmp_serde::to_vec(&Response {
					book: (buys, sells),
					orders: their_orders,
					matches: msg.0.matches.iter().rev()
						.map(|i| VisibleOrder { qty: i.qty, price: i.price, mine: i.seller == tid || i.buyer == tid } ).collect(),
					dark_matches: msg.1.matches.iter().rev()
						.map(|i| VisibleOrder { qty: i.qty, price: i.price, mine: i.seller == tid || i.buyer == tid } ).collect(),
					dark_orders: their_dark_orders,
					status: user.clone()
				}).unwrap()				
				};
                // Send back to person who opened the websocket
                if sink.send(Message::Binary(mesg)).await.is_err() {
                    break;
                }
            }
        });
    })
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }	

	let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // invoke the default handler and exit the process
        orig_hook(panic_info);
        std::process::exit(1);
    }));
	
    tracing_subscriber::fmt::init();
    let app = Route::new().at(
        "/",
        get(ws.data(tokio::sync::broadcast::channel::<(Book, Book)>(32).0)),
    );
    Server::new(TcpListener::bind("0.0.0.0:5001")).run(app).await
}
