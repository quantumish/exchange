use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, Document};

pub fn get_element(document: &Document, id: &str) -> HtmlElement {
	document.get_element_by_id(id).unwrap()
		.dyn_into::<web_sys::HtmlElement>().unwrap()
}

pub fn table_push_row(table: &web_sys::HtmlTableElement, val1: &str, val2: &str) {
	table.insert_row().unwrap();
	let rows = table.rows();
	let row = rows.item(rows.length()-1).unwrap()
		.dyn_into::<web_sys::HtmlTableRowElement>().unwrap();
	row.insert_cell().unwrap();
	row.insert_cell().unwrap();
	let cell1 = row.cells().item(0).unwrap().dyn_into::<web_sys::HtmlElement>().unwrap()
		.set_inner_html(&val1);
	let cell2 = row.cells().item(1).unwrap().dyn_into::<web_sys::HtmlElement>().unwrap()
		.set_inner_html(&val2);
}

pub fn rebuild_tables(vals: (Vec<(u64, [u8; 8])>, Vec<(u64, [u8; 8])>)) {
	let window = web_sys::window().expect("no global `window` exists");
	let document = window.document().expect("should have a document on window");

	let buy_elem = get_element(&document, "buys");
	buy_elem.set_inner_html("");
	let buy_table = buy_elem.dyn_into::<web_sys::HtmlTableElement>().unwrap();
	let sell_elem = get_element(&document, "sells");
	sell_elem.set_inner_html("");
	let sell_table = sell_elem.dyn_into::<web_sys::HtmlTableElement>().unwrap();
	table_push_row(&buy_table, "<b>Qty</b>", "<b>Price</b>");
	table_push_row(&sell_table, "<b>Qty</b>", "<b>Price</b>");

	for i in vals.0 {
		table_push_row(
			&buy_table,
			&i.0.to_string(),
			unsafe { &std::mem::transmute::<[u8; 8], f64>(i.1).to_string() } 
		);
	}
	for i in vals.1 {
		table_push_row(
			&sell_table,
			&i.0.to_string(),
			unsafe { &std::mem::transmute::<[u8; 8], f64>(i.1).to_string() } 
		);
	}
}

#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
	let window = web_sys::window().expect("no global `window` exists");
	let document = window.document().expect("should have a document on window");

	let ws = web_sys::WebSocket::new("ws://localhost:3001")?;
    let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: web_sys::MessageEvent| {
    	if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
			let alloced_text: String = txt.into();
			let vals: (Vec<(u64, [u8; 8])>, Vec<(u64, [u8; 8])>) = 
				serde_json_wasm::from_str(&alloced_text).unwrap();
			
			rebuild_tables(vals);
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
		let order = (otype, qty, unsafe {std::mem::transmute::<f64, [u8; 8]>(price)});				
		let msg = serde_json_wasm::to_string(&order).unwrap();		
		cloned_ws.send_with_str(&msg).unwrap();
	}) as Box<dyn FnMut()>);
	document.get_element_by_id("submit-button").unwrap()
		.dyn_into::<web_sys::HtmlButtonElement>().unwrap()
		.set_onclick(Some(submit.as_ref().unchecked_ref()));
	submit.forget();
	Ok(())
}
