//! Bentuk data dari GET /api/v2/myTrades (field sesuai data nyata Fase 0).
use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trade {
    pub trade_id: String,
    pub order_id: String,
    pub symbol: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub qty: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub quote_qty: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub commission: Decimal,
    pub commission_asset: String,
    pub is_buyer: bool,
    #[serde(default)]
    pub is_maker: bool,
    /// milidetik sejak epoch
    pub time: i64,
}

#[derive(Debug, Deserialize)]
pub struct MyTradesRespons {
    pub data: Vec<Trade>,
}

pub fn parse_my_trades(json: &str) -> Result<Vec<Trade>, crate::Error> {
    serde_json::from_str::<MyTradesRespons>(json)
        .map(|r| r.data)
        .map_err(|e| crate::Error::Json(e.to_string()))
}
