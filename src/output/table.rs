use comfy_table::{Cell, Color, ContentArrangement, Table};
use serde_json::Value;

/// Auto-detect the response shape and print a suitable table.
pub fn print_auto(label: &str, value: &Value) {
    match value {
        Value::Array(arr) if !arr.is_empty() => {
            if arr[0].is_object() {
                print_object_array(arr);
            } else {
                // Simple array of values
                println!("{}", serde_json::to_string_pretty(value).unwrap_or_default());
            }
        }
        Value::Object(map) => {
            // Check for nested data patterns
            if let Some(data) = map.get("data") {
                if data.is_array() {
                    print_auto(label, data);
                    return;
                }
                if data.is_object() {
                    print_key_value(label, data);
                    return;
                }
            }

            // Check for balances array inside account info
            if let Some(balances) = map.get("balances") {
                if balances.is_array() {
                    print_balances(balances);
                    return;
                }
            }

            // Check for bids/asks (orderbook)
            if map.contains_key("bids") || map.contains_key("asks") || map.contains_key("buys") {
                print_orderbook(value);
                return;
            }

            // Generic key-value table
            print_key_value(label, value);
        }
        _ => {
            println!("{}", serde_json::to_string_pretty(value).unwrap_or_default());
        }
    }
}

/// Print an array of JSON objects as a table.
fn print_object_array(arr: &[Value]) {
    if arr.is_empty() {
        println!("(no data)");
        return;
    }

    // Collect column headers from first object
    let headers: Vec<String> = if let Some(obj) = arr[0].as_object() {
        obj.keys().cloned().collect()
    } else {
        return;
    };

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);

    // Header row
    let header_cells: Vec<Cell> = headers
        .iter()
        .map(|h| Cell::new(h).fg(Color::Cyan))
        .collect();
    table.set_header(header_cells);

    // Data rows
    for item in arr {
        let row: Vec<Cell> = headers
            .iter()
            .map(|h| {
                let val = item.get(h).unwrap_or(&Value::Null);
                Cell::new(format_value(val))
            })
            .collect();
        table.add_row(row);
    }

    println!("{table}");
}

/// Print a JSON object as key-value pairs.
fn print_key_value(label: &str, value: &Value) {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        Cell::new("Field").fg(Color::Cyan),
        Cell::new("Value").fg(Color::Cyan),
    ]);

    if let Some(obj) = value.as_object() {
        for (key, val) in obj {
            // Skip nested objects/arrays for the flat view
            if val.is_object() || val.is_array() {
                let summary = match val {
                    Value::Array(a) => format!("[{} items]", a.len()),
                    Value::Object(o) => format!("{{{} fields}}", o.len()),
                    _ => format_value(val),
                };
                table.add_row(vec![
                    Cell::new(key).fg(Color::Green),
                    Cell::new(summary),
                ]);
            } else {
                table.add_row(vec![
                    Cell::new(key).fg(Color::Green),
                    Cell::new(format_value(val)),
                ]);
            }
        }
    }

    if !label.is_empty() {
        use colored::Colorize;
        println!("{}", label.bold());
    }
    println!("{table}");
}

/// Print balances in a clean table (only non-zero balances).
fn print_balances(value: &Value) {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        Cell::new("Asset").fg(Color::Cyan),
        Cell::new("Free").fg(Color::Cyan),
        Cell::new("Locked").fg(Color::Cyan),
    ]);

    if let Some(arr) = value.as_array() {
        for item in arr {
            let free = item["free"].as_str().unwrap_or("0");
            let locked = item["locked"].as_str().unwrap_or("0");

            // Only show non-zero balances
            let free_f: f64 = free.parse().unwrap_or(0.0);
            let locked_f: f64 = locked.parse().unwrap_or(0.0);
            if free_f == 0.0 && locked_f == 0.0 {
                continue;
            }

            let asset = item["asset"].as_str().unwrap_or("?");
            table.add_row(vec![
                Cell::new(asset).fg(Color::Yellow),
                Cell::new(free).fg(Color::Green),
                Cell::new(locked).fg(if locked_f > 0.0 {
                    Color::Red
                } else {
                    Color::White
                }),
            ]);
        }
    }

    use colored::Colorize;
    println!("{}", "Account Balances".bold());
    println!("{table}");
}

/// Print orderbook (bids/asks).
fn print_orderbook(value: &Value) {
    use colored::Colorize;
    println!("{}", "Order Book".bold());

    // Asks (sell side)
    let asks_key = if value.get("asks").is_some() {
        "asks"
    } else {
        "asks"
    };
    let bids_key = if value.get("bids").is_some() {
        "bids"
    } else {
        "buys"
    };

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        Cell::new("Price").fg(Color::Cyan),
        Cell::new("Quantity").fg(Color::Cyan),
        Cell::new("Side").fg(Color::Cyan),
    ]);

    // Print asks (reversed so lowest ask is near the spread)
    if let Some(asks) = value.get(asks_key).and_then(|v| v.as_array()) {
        let mut ask_rows: Vec<_> = asks
            .iter()
            .filter_map(|a| {
                let arr = a.as_array()?;
                Some((
                    arr.first()?.as_str().unwrap_or("?").to_string(),
                    arr.get(1)?.as_str().unwrap_or("?").to_string(),
                ))
            })
            .collect();
        ask_rows.reverse();

        for (price, qty) in &ask_rows {
            table.add_row(vec![
                Cell::new(price).fg(Color::Red),
                Cell::new(qty),
                Cell::new("ASK").fg(Color::Red),
            ]);
        }
    }

    // Spread separator
    table.add_row(vec![
        Cell::new("───────").fg(Color::DarkGrey),
        Cell::new("───────").fg(Color::DarkGrey),
        Cell::new("SPREAD").fg(Color::DarkGrey),
    ]);

    // Print bids
    if let Some(bids) = value.get(bids_key).and_then(|v| v.as_array()) {
        for bid in bids {
            if let Some(arr) = bid.as_array() {
                let price = arr.first().and_then(|v| v.as_str()).unwrap_or("?");
                let qty = arr.get(1).and_then(|v| v.as_str()).unwrap_or("?");
                table.add_row(vec![
                    Cell::new(price).fg(Color::Green),
                    Cell::new(qty),
                    Cell::new("BID").fg(Color::Green),
                ]);
            }
        }
    }

    println!("{table}");
}

/// Format a JSON value for display in a table cell.
fn format_value(val: &Value) -> String {
    match val {
        Value::Null => "—".to_string(),
        Value::Bool(b) => if *b { "✓" } else { "✗" }.to_string(),
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => val.to_string(),
    }
}
