mod book;
use book::*;

/// Currently a modified version of `poem`'s default websocket-chat example
use futures_util::{SinkExt, StreamExt};
use poem::{
    get, handler,
    listener::TcpListener,
    web::{
        websocket::{Message, WebSocket},
        Data, Path,
    },
    EndpointExt, IntoResponse, Route, Server,
};

static LOB: std::sync::Mutex<Option<Book>> = std::sync::Mutex::new(None);
pub fn order(order: Order) {
    LOB
        .lock()
        .unwrap()
        .get_or_insert_with(|| Book::new())
		.add_order(order)
}


pub fn gen_id() -> i64 {
    static STATE: std::sync::Mutex<Option<rustflake::Snowflake>> = std::sync::Mutex::new(None);

    STATE
        .lock()
        .unwrap()
        .get_or_insert_with(|| rustflake::Snowflake::new(1_564_790_400_000, 2, 1))
        .generate()
}


#[handler]
fn ws(
    ws: WebSocket,
    sender: Data<&tokio::sync::broadcast::Sender<String>>,
) -> impl IntoResponse {
    let sender = sender.clone(); // Subscribe to global channel
    ws.on_upgrade(move |socket| async move {
		let tid = gen_id();
        let mut receiver = sender.subscribe();
        let (mut sink, mut stream) = socket.split();

        tokio::spawn(async move {
            // Wait to receive a message from person who opened websocket
            while let Some(Ok(mesg)) = stream.next().await {
                if let Message::Text(text) = mesg {
					let duration_since_epoch = std::time::SystemTime::now()
						.duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap();
					let vals: (&str, u64, [u8; 8]) = serde_json::from_str(&text).unwrap();

					let o = Order {
						otype: match vals.0 {
							"buy" => OrderType::Bid,
							"sell" => OrderType::Ask,
							&_ => todo!(), 
						},
						price: unsafe { std::mem::transmute::<[u8; 8], f64>(vals.2) },
						trader: tid,
						qty: vals.1,
						time: duration_since_epoch.as_nanos(),
						id: gen_id(),
						hidden: false
					};
					
					order(o);

					let mut book = LOB.lock().unwrap();
					let tmp = book.get_or_insert_with(|| Book::new());
					let buys: Vec<(u64, [u8; 8])> = tmp.buy.iter().map(|o| {
						(o.qty, unsafe {std::mem::transmute::<f64, [u8; 8]>(o.price)})
					}).collect();
					let sells: Vec<(u64, [u8; 8])> = tmp.sell.iter().map(|o| {
						(o.qty, unsafe {std::mem::transmute::<f64, [u8; 8]>(o.price)})
					}).collect();
                    
                    // Send to global channel
                    if sender.send(serde_json::to_string(&(buys, sells)).unwrap()).is_err() {
                        break;
                    }
                }
            }
        });

        tokio::spawn(async move {
            // Wait to receive a message from global channel
            while let Ok(msg) = receiver.recv().await {
                // Send back to person who opened the websocket
                if sink.send(Message::Text(msg)).await.is_err() {
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
    tracing_subscriber::fmt::init();
    let app = Route::new().at(
        "/",
        get(ws.data(tokio::sync::broadcast::channel::<String>(32).0)),
    );
    Server::new(TcpListener::bind("127.0.0.1:3001")).run(app).await
}
