use clap::Subcommand;

use crate::errors::BittimeError;
use crate::output::CommandOutput;
use crate::{normalize_pair, AppContext};

#[derive(Debug, Subcommand)]
pub enum OrderCommand {
    /// Place a buy order
    Buy {
        /// Trading pair symbol (e.g., USDTIDR, BTCUSDT)
        pair: String,

        /// Order type: LIMIT or MARKET
        #[arg(short = 't', long, default_value = "LIMIT")]
        r#type: String,

        /// Order price (required for LIMIT orders)
        #[arg(short, long)]
        price: Option<String>,

        /// Order quantity
        #[arg(short = 'v', long)]
        volume: String,

        /// Client order ID (optional)
        #[arg(long)]
        client_order_id: Option<String>,
    },

    /// Place a sell order
    Sell {
        /// Trading pair symbol
        pair: String,

        /// Order type: LIMIT or MARKET
        #[arg(short = 't', long, default_value = "LIMIT")]
        r#type: String,

        /// Order price (required for LIMIT orders)
        #[arg(short, long)]
        price: Option<String>,

        /// Order quantity
        #[arg(short = 'v', long)]
        volume: String,

        /// Client order ID (optional)
        #[arg(long)]
        client_order_id: Option<String>,
    },

    /// Cancel an active order
    Cancel {
        /// Trading pair symbol
        pair: String,

        /// Order ID to cancel
        #[arg(long)]
        order_id: String,
    },

    /// Query a specific order's status
    Query {
        /// Trading pair symbol
        pair: String,

        /// Order ID to query
        #[arg(long)]
        order_id: String,
    },

    /// List current open orders
    OpenOrders {
        /// Trading pair symbol
        pair: String,

        /// Maximum number of orders (default: 1000)
        #[arg(short, long, default_value = "1000")]
        count: u32,
    },

    /// List all orders (active, canceled, filled)
    AllOrders {
        /// Trading pair symbol
        pair: String,

        /// Get orders >= this order ID
        #[arg(long)]
        order_id: Option<String>,
    },

    /// List pending orders (alias for open orders on Bittime)
    PendingOrders {
        /// Trading pair symbol
        pair: String,
    },

    /// Show public order book depth for a symbol
    BookOrders {
        /// Trading pair symbol
        pair: String,

        /// Maximum number of orders (default: 1000)
        #[arg(short, long, default_value = "1000")]
        count: u32,
    },

    /// Execute a convert trade
    Convert {
        /// Trading pair symbol
        pair: String,
    },
}

impl OrderCommand {
    pub async fn execute(&self, ctx: &AppContext) -> Result<CommandOutput, BittimeError> {
        let client = &ctx.client;

        let output = match self {
            Self::Buy {
                pair,
                r#type,
                price,
                volume,
                client_order_id,
            } => {
                self.place_order(
                    ctx,
                    pair,
                    "BUY",
                    r#type,
                    price.as_deref(),
                    volume,
                    client_order_id.as_deref(),
                )
                .await?
            }

            Self::Sell {
                pair,
                r#type,
                price,
                volume,
                client_order_id,
            } => {
                self.place_order(
                    ctx,
                    pair,
                    "SELL",
                    r#type,
                    price.as_deref(),
                    volume,
                    client_order_id.as_deref(),
                )
                .await?
            }

            Self::Cancel { pair, order_id } => {
                let sym = crate::normalize_pair(pair);
                let result = client
                    .delete_signed(
                        "/api/v1/order",
                        &[("symbol", sym.as_str()), ("orderId", order_id.as_str())],
                    )
                    .await?;
                CommandOutput::new(result, "Cancel Result")
                    .with_addendum(format!("Order {} cancelled", order_id))
            }

            Self::Query { pair, order_id } => {
                let sym = crate::normalize_pair(pair);
                let result = client
                    .get_signed(
                        "/api/v1/order",
                        &[("symbol", sym.as_str()), ("orderId", order_id.as_str())],
                    )
                    .await?;
                CommandOutput::new(result, format!("Order {} — {}", order_id, sym))
            }

            Self::OpenOrders { pair, count } => {
                let sym = crate::normalize_pair(pair);
                let lim = count.to_string();
                let result = client
                    .get_signed(
                        "/api/v1/openOrders",
                        &[("symbol", sym.as_str()), ("limit", lim.as_str())],
                    )
                    .await?;
                CommandOutput::new(result, format!("Open Orders — {}", sym))
            }

            Self::AllOrders { pair, order_id } => {
                let sym = crate::normalize_pair(pair);
                let oid = order_id.as_deref().unwrap_or("");
                let result = client
                    .get_signed(
                        "/api/v1/allOrders",
                        &[("symbol", sym.as_str()), ("orderId", oid)],
                    )
                    .await?;
                CommandOutput::new(result, format!("All Orders — {}", sym))
            }

            Self::PendingOrders { pair } => {
                let sym = crate::normalize_pair(pair);
                let result = client
                    .get_signed("/api/v1/openOrders", &[("symbol", sym.as_str())])
                    .await?;
                CommandOutput::new(result, format!("Pending Orders — {}", sym))
            }

            Self::BookOrders { pair, count } => {
                let sym = crate::normalize_pair(pair);
                let lim = count.to_string();
                let result = client
                    .get_public(
                        "/api/v1/depth",
                        &[("symbol", sym.as_str()), ("limit", lim.as_str())],
                    )
                    .await?;
                CommandOutput::new(result, format!("Order Book — {}", sym))
            }

            Self::Convert { pair } => {
                let sym = crate::normalize_pair(pair);
                let result = client
                    .post_signed("/api/convert/trades", &[("symbol", sym.as_str())])
                    .await?;
                CommandOutput::new(result, format!("Convert — {}", sym))
            }
        };

        Ok(output.with_format(ctx.format))
    }

    async fn place_order(
        &self,
        ctx: &AppContext,
        symbol: &str,
        side: &str,
        order_type: &str,
        price: Option<&str>,
        quantity: &str,
        client_order_id: Option<&str>,
    ) -> Result<CommandOutput, BittimeError> {
        let sym = crate::normalize_pair(symbol);
        let otype = order_type.to_uppercase();
        let coid = client_order_id
            .map(|s| s.to_string())
            .unwrap_or_else(|| chrono::Utc::now().timestamp().to_string());

        let price_str = price.unwrap_or("0");

        let params: Vec<(&str, &str)> = vec![
            ("symbol", sym.as_str()),
            ("side", side),
            ("type", otype.as_str()),
            ("price", price_str),
            ("quantity", quantity),
            ("newClientOrderId", coid.as_str()),
        ];

        let result = ctx.client.post_signed("/api/v1/order", &params).await?;

        let mut output = CommandOutput::new(result.clone(), "Order Result");
        if let Some(order_id) = result.get("orderId") {
            output = output.with_addendum(format!(
                "{} {} {} @ {} — Order ID: {}",
                side, quantity, sym, price_str, order_id
            ));
        }

        Ok(output.with_format(ctx.format))
    }
}
