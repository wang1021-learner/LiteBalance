//! 外部食品数据库与条形码查询客户端模块
//!
//! 包含 GS1 模 10 条形码解析器、OpenFoodFacts (OFF) API 客户端、
//! 自定义 User-Agent 封装、客户端本地滑动窗口限速器、以及 ODbL 协议合规署名。

pub mod barcode;
pub mod remote_food_client;

pub use barcode::{BarcodeError, BarcodeType, BarcodeValidator, NormalizedBarcode};
pub use remote_food_client::{
    RemoteClientError, RemoteFoodClient, RemoteFoodProduct, SlidingWindowRateLimiter,
    DEFAULT_USER_AGENT, OFF_API_BASE_URL,
};
