use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, Document};
use plotters::prelude::*;
use plotters_canvas::CanvasBackend;

pub fn pretty_u64(x: u64) -> String {
	if x < 1000 {
		return format!("{}", x);
	} else if x < 1_000_000{
		return format!("{:.2}k", x/1000);
	} else {
		return format!("{:.2}M", x/1_000_000);
	}
}

pub fn get_element(document: &Document, id: &str) -> HtmlElement {
	document.get_element_by_id(id).unwrap()
		.dyn_into::<web_sys::HtmlElement>().unwrap()
}

pub fn table_push_row(table: &web_sys::HtmlTableElement, val1: &str, val2: &str, mine: bool) {
	table.insert_row().unwrap();
	let rows = table.rows();
	let row = rows.item(rows.length()-1).unwrap()
		.dyn_into::<web_sys::HtmlTableRowElement>().unwrap();
	if mine { row.set_class_name("mine"); }
	row.insert_cell().unwrap();
	row.insert_cell().unwrap();
	let cell1 = row.cells().item(0).unwrap().dyn_into::<web_sys::HtmlElement>().unwrap()
		.set_inner_html(&val1);
	let cell2 = row.cells().item(1).unwrap().dyn_into::<web_sys::HtmlElement>().unwrap()
		.set_inner_html(&val2);
}

pub fn rebuild_tables(vals: (Vec<common::VisibleOrder>, Vec<common::VisibleOrder>), matches: Vec<common::VisibleOrder>, dark_matches: Vec<common::VisibleOrder>) {
	let window = web_sys::window().expect("no global `window` exists");
	let document = window.document().expect("should have a document on window");

	let buy_elem = get_element(&document, "buys");
	buy_elem.set_inner_html("");
	let buy_table = buy_elem.dyn_into::<web_sys::HtmlTableElement>().unwrap();
	buy_table.create_caption();
	buy_table.caption().unwrap().set_inner_text("Bids");	
	
	let sell_elem = get_element(&document, "sells");
	sell_elem.set_inner_html("");
	let sell_table = sell_elem.dyn_into::<web_sys::HtmlTableElement>().unwrap();
	sell_table.create_caption();
	sell_table.caption().unwrap().set_inner_text("Offers");
	
	
	table_push_row(&buy_table, "<b>qty</b>", "<b>price</b>", false);
	table_push_row(&sell_table, "<b>price</b>", "<b>qty</b>", false);

	let len = std::cmp::max(vals.0.len(), vals.1.len());
	let len = std::cmp::max(len, 11);
	
	for i in 0..len {
		let buy = vals.0.get(i);
		let sell = vals.1.get(i);
		if let Some(o) = buy {
			table_push_row(&buy_table, &o.qty.to_string(), &o.price.to_string(), o.mine);
		} else { table_push_row(&buy_table, "&nbsp;", "&nbsp;", false); }
		if let Some(o) = sell {
			table_push_row(&sell_table, &o.price.to_string(), &o.qty.to_string(), o.mine);
		} else { table_push_row(&sell_table, "&nbsp;", "&nbsp;", false); }
	}

	let match_elem = get_element(&document, "matches");
	match_elem.set_inner_html("");
	let match_table = match_elem.dyn_into::<web_sys::HtmlTableElement>().unwrap();
	match_table.create_caption();
	table_push_row(&match_table, "<b>price</b>", "<b>qty</b>", false);
	for i in 0..std::cmp::max(len, matches.len()) {		
		let m = matches.get(i);
		if let Some(o) = m {
			table_push_row(&match_table, &o.price.to_string(), &o.qty.to_string(), o.mine);
		} else { table_push_row(&match_table, "&nbsp;", "&nbsp;", false); }
	}

	
	let dark_match_elem = get_element(&document, "dark-matches");
	dark_match_elem.set_inner_html("");
	let dark_match_table = dark_match_elem.dyn_into::<web_sys::HtmlTableElement>().unwrap();
	dark_match_table.create_caption();
	table_push_row(&dark_match_table, "<b>price</b>", "<b>qty</b>", false);
	for i in 0..std::cmp::max(11, dark_matches.len()) {
		let m = dark_matches.get(i);
		if let Some(o) = m {
			table_push_row(&dark_match_table, &o.price.to_string(), &o.qty.to_string(), false);
		} else { table_push_row(&dark_match_table, "&nbsp;", "&nbsp;", false); }
	}
}

#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
	let window = web_sys::window().expect("no global `window` exists");
	let document = window.document().expect("should have a document on window");

	let ws = web_sys::WebSocket::new("ws://localhost:5001")?;
	ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

	let cloned_ws = ws.clone();
	let onopen_callback = Closure::<dyn FnMut(_)>::new(move |_e: web_sys::MessageEvent| {
		let req = common::Request::Get;
		let msg = rmp_serde::to_vec(&req).unwrap();
		cloned_ws.send_with_u8_array(&msg).unwrap();
	});	 
	ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));	
	onopen_callback.forget();
	
    let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: web_sys::MessageEvent| {
    	if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
			let arr = js_sys::Uint8Array::new(&abuf);			
			let res: common::Response = rmp_serde::from_slice(&arr.to_vec()).unwrap();				

			let book = res.book;
			rebuild_tables(book.clone(), res.matches.clone(), res.dark_matches.clone());			
			let doc = web_sys::window().unwrap().document().unwrap();
			get_element(&doc, "goal").set_inner_html(&format!(
				"<b>My status:</b> {} <br>\
                Goal: {} {} shares.<br>\
                Able to <i>{}</i> up to {} shares.<br><br>\
                Progress: <span style='color: {}'>{}/{}</span>\
                &nbsp(opp. <span style='color: {}'>{}/{}</span>)<br>
                # of matches so far: {}<br><br>
                My VWAP: {:.3}",
				if res.status.done >= res.status.amount { "<code style='color: green; font-size: 12pt'>DONE</code>" } else { "" },
				if res.status.goal == common::OrderType::Bid { "buy" } else { "sell" },
				pretty_u64(res.status.amount),
				if res.status.goal == common::OrderType::Bid { "sell" } else { "buy" },
				pretty_u64(res.status.tolerance),
				if res.status.done >= res.status.amount { "green" } else { "inherit" },
				pretty_u64(res.status.done), pretty_u64(res.status.amount),
				if res.status.opp >= res.status.tolerance { "red" } else { "inherit" },
				pretty_u64(res.status.opp), pretty_u64(res.status.tolerance),
				res.status.orders,
				res.matches.iter().filter(|i| i.mine).map(|i| i.qty as f64 * i.price).sum::<f64>()/
					res.matches.iter().filter(|i| i.mine).map(|i| i.qty as f64).sum::<f64>()
			));
			let h = get_element(&doc, "buys").offset_height();
			let graph = get_element(&doc, "lob-graph")
				.dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
			graph.set_height(std::cmp::max(300, h as u32));
			if let Some(p) = res.matches.first() {				
				get_element(&doc, "exchange-last-price").set_inner_html(
					&format!("Last price: {}<br>VWAP: {:.3}", p.price,
							 res.matches.iter().map(|i| i.qty as f64 * i.price).sum::<f64>()/
							 res.matches.iter().map(|i| i.qty as f64).sum::<f64>(),
					)
				);
			}
			if let Some(p) = res.dark_matches.first() {				
				get_element(&doc, "darkpool-last-price").set_inner_html(
					&format!("Last price: {}<br>VWAP: {:.3}", p.price,
							 res.dark_matches.iter().map(|i| i.qty as f64 * i.price).sum::<f64>()/
							 res.dark_matches.iter().map(|i| i.qty as f64).sum::<f64>(),
					)
				);
			}

			get_element(&doc, "your-orders").set_inner_html(&
				res.orders.iter().map(|o| format!(
					"<div>\
	                   <button class='close-button' type='button' data-close>\
                         <span aria-hidden='true'>&times;</span>\
	                   </button>\
	                   &nbsp; {} {} AT {} {}\
                    </div>",
					match o.otype {
						common::OrderType::Ask => "SELL",
						common::OrderType::Bid => "BUY",
					},
					o.qty,
					o.price,
					if o.hidden { "(hidden)" } else { "" }
				)).fold(String::from("<b>My orders:</b><br>"), |a, b| a + &b + "\n"));


			get_element(&doc, "your-dark-orders").set_inner_html(&
				res.dark_orders.iter().map(|o| format!(
					"<div>\
	                   <button class='close-button' type='button' data-close>\
                         <span aria-hidden='true'>&times;</span>\
	                   </button>\
	                   &nbsp; {} {} AT {}\
                    </div>",
					match o.otype {
						common::OrderType::Ask => "SELL",
						common::OrderType::Bid => "BUY",
					},
					o.qty,
					o.price					
				)).fold(String::from("<b>My orders:</b><br>"), |a, b| a + &b + "\n"));

			
			let children = get_element(&doc, "your-orders").children();		
			web_sys::console::log_1(&JsValue::from_str(&format!("{:?}", res.orders)));
			for i in 0..children.length()-2 {
				web_sys::console::log_1(&JsValue::from_str(&format!("{}", i)));
				let id = res.orders[i as usize].id;
				web_sys::console::log_1(&JsValue::from_str("huh"));
				let cancel = Closure::wrap(Box::new(move || {
					// fuck it. open the same websocket inside a websocket message handler.
					// i don't care anymore.
					let ws = web_sys::WebSocket::new("ws://localhost:5001").unwrap();
					ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
					let cloned_ws = ws.clone();
					let onopen_callback = Closure::<dyn FnMut(_)>::new(move |_e: web_sys::MessageEvent| {
						let req = common::Request::Cancel(id);
						let msg = rmp_serde::to_vec(&req).unwrap();
						cloned_ws.send_with_u8_array(&msg).unwrap();
					});	 
					ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));	
					onopen_callback.forget();					
				}) as Box<dyn FnMut()>);

				web_sys::console::log_1(&JsValue::from_str(&format!("len {:?}", children.length())));
				web_sys::console::log_1(&JsValue::from_str(&format!("i {}", i+2)));
				children.item(i+2).unwrap().dyn_into::<HtmlElement>().unwrap()
					.first_element_child().unwrap()
					.dyn_into::<web_sys::HtmlButtonElement>().unwrap()
					.set_onclick(Some(cancel.as_ref().unchecked_ref()));
				cancel.forget();
			}

			let backend = CanvasBackend::new("lob-graph").expect("cannot find canvas");
			let root = backend.into_drawing_area();

			root.fill(&WHITE).unwrap();

			let buys: Vec<f64> = book.0.iter().map(|x| x.price).collect();
			let sells: Vec<f64> = book.1.iter().map(|x| -x.price).collect();				

			let min = sells.clone().into_iter()
				.min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(0.0);
			let max = buys.clone().into_iter()
				.max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(0.0);			
			
			let mut ctx = ChartBuilder::on(&root)				
				.set_label_area_size(LabelAreaPosition::Bottom, 20)
				.build_cartesian_2d(min..max,
					-(sells.len() as f64)..(buys.len() as f64))
				.unwrap();

			ctx.configure_mesh().draw().unwrap();
			ctx.draw_series((0..).zip(buys.iter()).map(|(y, x)| {
				let mut bar = Rectangle::new([
					(0.0, y as f64), 
					(*x, y as f64 + 1.0)
				], GREEN.filled());
				bar.set_margin(5, 5, 0, 0);
				bar
			})).unwrap();			
			ctx.draw_series(
				(-(sells.len() as i32)..0)
					.map(|i| i as f64)
					.rev()
					.zip(sells.iter())
					.map(|(y, x)| {
						let mut bar = Rectangle::new([
							(0.0, y), 		
							(*x, y + 1.0)
						], RED.filled());
						bar.set_margin(5, 5, 0, 0);
						bar
					})
			).unwrap();

			let backend = CanvasBackend::new("price-graph").expect("cannot find canvas");
			let root = backend.into_drawing_area();

			root.fill(&WHITE).unwrap();

			let prices: Vec<f64> = res.matches.iter().rev().map(|x| x.price).collect();

			let min = prices.clone().into_iter()
				.min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(0.0);
			let max = prices.clone().into_iter()
				.max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(0.0);			
			
			let mut ctx = ChartBuilder::on(&root)				
				.set_label_area_size(LabelAreaPosition::Left, 40)
				.build_cartesian_2d(0..prices.len()-1, min-(0.1*min)..max+(0.1*max))
				.unwrap();

			ctx.configure_mesh().draw().unwrap();
			ctx.draw_series(				
				LineSeries::new((0..prices.len()).zip(prices), &BLACK)
			).unwrap();

			let backend = CanvasBackend::new("darkpool-price-graph").expect("cannot find canvas");
			let root = backend.into_drawing_area();

			root.fill(&BLACK).unwrap();

			let prices: Vec<f64> = res.dark_matches.iter().rev().map(|x| x.price).collect();

			let min = prices.clone().into_iter()
				.min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(0.0);
			let max = prices.clone().into_iter()
				.max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(0.0);			
			
			let mut ctx = ChartBuilder::on(&root)				
				.set_label_area_size(LabelAreaPosition::Left, 40)
				.build_cartesian_2d(0..prices.len()-1, min-(0.1*min)..max+(0.1*max))
				.unwrap();

			ctx.configure_mesh().draw().unwrap();
			ctx.draw_series(				
				LineSeries::new((0..prices.len()).zip(prices), &WHITE)
			).unwrap();

        }
    });
    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();
    
	let cloned_ws = ws.clone();
	let submit = Closure::wrap(Box::new(move || {
		let doc = web_sys::window().unwrap().document().unwrap();
		let otype = get_element(&doc, "typeselector")
			.dyn_into::<web_sys::HtmlSelectElement>().unwrap()
			.value();
		let qty = str::parse::<u64>(
			&get_element(&doc, "qtyinput").dyn_into::<web_sys::HtmlInputElement>().unwrap().value()
		).unwrap();
		let price = str::parse::<f64>(
			&get_element(&doc, "priceinput").dyn_into::<web_sys::HtmlInputElement>().unwrap().value()
		).unwrap();
		let hidden = get_element(&doc,"is-hidden")
			.dyn_into::<web_sys::HtmlInputElement>().unwrap().checked();
		web_sys::console::log_1(&JsValue::from_bool(hidden));
		let req = common::Request::ExchangeOrder(common::OrderReq {
			kind: match otype.as_str() {
				"buy" => common::OrderType::Bid,
				"sell" => common::OrderType::Ask,
				&_ => todo!()
			},
			price,
			qty,
			hidden
		});		
		let msg = rmp_serde::to_vec(&req).unwrap();
		cloned_ws.send_with_u8_array(&msg).unwrap();
	}) as Box<dyn FnMut()>);
	let cloned_ws = ws.clone();
	let dark_submit = Closure::wrap(Box::new(move || {
		let doc = web_sys::window().unwrap().document().unwrap();
		let otype = get_element(&doc, "typeselector")
			.dyn_into::<web_sys::HtmlSelectElement>().unwrap()
			.value();
		let qty = str::parse::<u64>( 
			&get_element(&doc, "qtyinput").dyn_into::<web_sys::HtmlInputElement>().unwrap().value()
		).unwrap();
		let price = str::parse::<f64>(
			&get_element(&doc, "priceinput").dyn_into::<web_sys::HtmlInputElement>().unwrap().value()
		).unwrap();
		let hidden = get_element(&doc,"is-hidden")
			.dyn_into::<web_sys::HtmlInputElement>().unwrap().checked();
		web_sys::console::log_1(&JsValue::from_bool(hidden));
		let req = common::Request::DarkpoolOrder(common::OrderReq {
			kind: match otype.as_str() {
				"buy" => common::OrderType::Bid,
				"sell" => common::OrderType::Ask,
				&_ => todo!()
			},
			price,
			qty,
			hidden
		});		
		let msg = rmp_serde::to_vec(&req).unwrap();
		cloned_ws.send_with_u8_array(&msg).unwrap();
	}) as Box<dyn FnMut()>);

	document.get_element_by_id("exchange-submit-button").unwrap()
		.dyn_into::<web_sys::HtmlButtonElement>().unwrap()
		.set_onclick(Some(submit.as_ref().unchecked_ref()));
	document.get_element_by_id("darkpool-submit-button").unwrap()
		.dyn_into::<web_sys::HtmlButtonElement>().unwrap()
		.set_onclick(Some(dark_submit.as_ref().unchecked_ref()));	
	submit.forget();
	dark_submit.forget();

	let darkpool = Closure::wrap(Box::new(move || {
		let doc = web_sys::window().unwrap().document().unwrap();
		doc.body().unwrap().set_class_name("dark");
		get_element(&doc, "header").set_inner_text("Darkpool");
		get_element(&doc, "darkpool").set_hidden(false);
		get_element(&doc, "exchange-button").set_hidden(false);
		get_element(&doc, "darkpool-submit-button").set_hidden(false);
		get_element(&doc, "darkpool-price-graph").set_hidden(false);
		get_element(&doc, "darkpool-last-price").set_hidden(false);
		get_element(&doc, "your-dark-orders").set_hidden(false);
		get_element(&doc, "outer-dark-matches").set_hidden(false);
		get_element(&doc, "your-orders").set_hidden(true);
		get_element(&doc, "outer-matches").set_hidden(true);
		get_element(&doc, "buys").set_hidden(true);
		get_element(&doc, "sells").set_hidden(true);
		get_element(&doc, "hidden").set_hidden(true);
		get_element(&doc, "exchange-last-price").set_hidden(true);
		get_element(&doc, "lob-graph").set_hidden(true);
		get_element(&doc, "price-graph").set_hidden(true);
		get_element(&doc, "darkpool-button").set_hidden(true); 
		get_element(&doc, "exchange-submit-button").set_hidden(true);
	}) as Box<dyn FnMut()>);
	document.get_element_by_id("darkpool-button").unwrap()
		.dyn_into::<web_sys::HtmlButtonElement>().unwrap()
		.set_onclick(Some(darkpool.as_ref().unchecked_ref()));	
	darkpool.forget();
	let exchange = Closure::wrap(Box::new(move || {
		let doc = web_sys::window().unwrap().document().unwrap();
		doc.body().unwrap().set_class_name("");
		get_element(&doc, "header").set_inner_text("Exchange");
		get_element(&doc, "darkpool").set_hidden(true);
		get_element(&doc, "exchange-button").set_hidden(true);
		get_element(&doc, "darkpool-submit-button").set_hidden(true);
		get_element(&doc, "darkpool-price-graph").set_hidden(true);
		get_element(&doc, "your-dark-orders").set_hidden(true);
		get_element(&doc, "darkpool-last-price").set_hidden(true);
		get_element(&doc, "outer-dark-matches").set_hidden(true);
		get_element(&doc, "your-orders").set_hidden(false);
		get_element(&doc, "hidden").set_hidden(false);
		get_element(&doc, "outer-matches").set_hidden(false);
		get_element(&doc, "buys").set_hidden(false);
		get_element(&doc, "sells").set_hidden(false);
		get_element(&doc, "exchange-last-price").set_hidden(false);
		get_element(&doc, "lob-graph").set_hidden(false);
		get_element(&doc, "price-graph").set_hidden(false);
		get_element(&doc, "darkpool-button").set_hidden(false); 
		get_element(&doc, "exchange-submit-button").set_hidden(false);
	}) as Box<dyn FnMut()>);
	document.get_element_by_id("exchange-button").unwrap()
		.dyn_into::<web_sys::HtmlButtonElement>().unwrap()
		.set_onclick(Some(exchange.as_ref().unchecked_ref()));	
	exchange.forget();

	
	Ok(())
}
