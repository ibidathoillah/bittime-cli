use clap::Subcommand;

use crate::client::BittimeClient;
use crate::errors::BittimeError;
use crate::output::{self, OutputFormat};

#[derive(Debug, Subcommand)]
pub enum TradeCommand {
    /// Place a buy order
    Buy {
        /// Trading pair symbol (e.g., USDTIDR, BTCUSDT)
        symbol: String,

        /// Order type: LIMIT or MARKET
        #[arg(short = 't', long, default_value = "LIMIT")]
        r#type: String,

        /// Order price (required for LIMIT orders)
        #[arg(short, long)]
        price: Option<String>,

        /// Order quantity
        #[arg(short, long)]
        quantity: String,

        /// Client order ID (optional)
        #[arg(long)]
        client_order_id: Option<String>,
    },

    /// Place a sell order
    Sell {
        /// Trading pair symbol
        symbol: String,

        /// Order type: LIMIT or MARKET
        #[arg(short = 't', long, default_value = "LIMIT")]
        r#type: String,

        /// Order price (required for LIMIT orders)
        #[arg(short, long)]
        price: Option<String>,

        /// Order quantity
        #[arg(short, long)]
        quantity: String,

        /// Client order ID (optional)
        #[arg(long)]
        client_order_id: Option<String>,
    },

    /// Cancel an active order
    Cancel {
        /// Trading pair symbol
        symbol: String,

        /// Order ID to cancel
        #[arg(long)]
        order_id: String,
    },

    /// Query a specific order's status
    Query {
        /// Trading pair symbol
        symbol: String,

        /// Order ID to query
        #[arg(long)]
        order_id: String,
    },

    /// List current open orders
    OpenOrders {
        /// Trading pair symbol
        symbol: String,

        /// Maximum number of orders (default: 1000)
        #[arg(short, long, default_value = "1000")]
        limit: u32,
    },

    /// List all orders (active, canceled, filled)
    AllOrders {
        /// Trading pair symbol
        symbol: String,

        /// Get orders >= this order ID
        #[arg(long)]
        order_id: Option<String>,
    },

    /// List pending orders
    PendingOrders {
        /// Trading pair symbol
        symbol: String,
    },

    /// List book orders
    BookOrders {
        /// Trading pair symbol
        symbol: String,

        /// Maximum number of orders (default: 1000)
        #[arg(short, long, default_value = "1000")]
        limit: u32,
    },

    /// Execute a convert trade
    Convert {
        /// Trading pair symbol
        symbol: String,
    },
}

impl TradeCommand {
    pub async fn execute(&self, client: &BittimeClient, format: OutputFormat) -> Result<(), BittimeError> {
        match self {
            Self::Buy {
                symbol,
                r#type,
                price,
                quantity,
                client_order_id,
            } => {
                self.place_order(client, format, symbol, "BUY", r#type, price.as_deref(), quantity, client_order_id.as_deref())
                    .await?;
            }

            Self::Sell {
                symbol,
                r#type,
                price,
                quantity,
                client_order_id,
            } => {
                self.place_order(client, format, symbol, "SELL", r#type, price.as_deref(), quantity, client_order_id.as_deref())
                    .await?;
            }

            Self::Cancel { symbol, order_id } => {
                let sym = symbol.to_uppercase();
                let result = client
                    .delete_signed(
                        "/api/v1/order",
                        &[("symbol", sym.as_str()), ("orderId", order_id.as_str())],
                    )
                    .await?;
                output::print_success(format, &format!("Order {} cancelled", order_id));
                output::render(format, "Cancel Result", &result);
            }

            Self::Query { symbol, order_id } => {
                let sym = symbol.to_uppercase();
                let result = client
                    .get_signed(
                        "/api/v1/order",
                        &[("symbol", sym.as_str()), ("orderId", order_id.as_str())],
                    )
                    .await?;
                output::render(format, &format!("Order {} — {}", order_id, sym), &result);
            }

            Self::OpenOrders { symbol, limit } => {
                let sym = symbol.to_uppercase();
                let lim = limit.to_string();
                let result = client
                    .get_signed(
                        "/api/v1/openOrders",
                        &[("symbol", sym.as_str()), ("limit", lim.as_str())],
                    )
                    .await?;
                output::render(format, &format!("Open Orders — {}", sym), &result);
            }

            Self::AllOrders { symbol, order_id } => {
                let sym = symbol.to_uppercase();
                let oid = order_id.as_deref().unwrap_or("");
                let result = client
                    .get_signed(
                        "/api/v1/allOrders",
                        &[("symbol", sym.as_str()), ("orderId", oid)],
                    )
                    .await?;
                output::render(format, &format!("All Orders — {}", sym), &result);
            }

            Self::PendingOrders { symbol } => {
                let sym = symbol.to_uppercase();
                let result = client
                    .get_signed("/api/v1/pendingOrders", &[("symbol", sym.as_str())])
                    .await?;
                output::render(format, &format!("Pending Orders — {}", sym), &result);
            }

            Self::BookOrders { symbol, limit } => {
                let sym = symbol.to_uppercase();
                let lim = limit.to_string();
                let result = client
                    .get_signed(
                        "/api/v1/bookOrders",
                        &[("symbol", sym.as_str()), ("limit", lim.as_str())],
                    )
                    .await?;
                output::render(format, &format!("Book Orders — {}", sym), &result);
            }

            Self::Convert { symbol } => {
                let sym = symbol.to_uppercase();
                let result = client
                    .post_signed("/api/convert/trades", &[("symbol", sym.as_str())])
                    .await?;
                output::render(format, &format!("Convert — {}", sym), &result);
            }
        }

        Ok(())
    }

    async fn place_order(
        &self,
        client: &BittimeClient,
        format: OutputFormat,
        symbol: &str,
        side: &str,
        order_type: &str,
        price: Option<&str>,
        quantity: &str,
        client_order_id: Option<&str>,
    ) -> Result<(), BittimeError> {
        let sym = symbol.to_uppercase();
        let otype = order_type.to_uppercase();
        let coid = client_order_id
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                chrono::Utc::now().timestamp().to_string()
            });

        let price_str = price.unwrap_or("0");

        let params: Vec<(&str, &str)> = vec![
            ("symbol", sym.as_str()),
            ("side", side),
            ("type", otype.as_str()),
            ("price", price_str),
            ("quantity", quantity),
            ("newClientOrderId", coid.as_str()),
        ];

        let result = client.post_signed("/api/v1/order", &params).await?;

        if let Some(order_id) = result.get("orderId") {
            output::print_success(
                format,
                &format!(
                    "{} {} {} @ {} — Order ID: {}",
                    side,
                    quantity,
                    sym,
                    price_str,
                    order_id
                ),
            );
        }
        output::render(format, "Order Result", &result);

        Ok(())
    }
}
