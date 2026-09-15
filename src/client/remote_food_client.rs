//! 外部食品数据库与条形码查询客户端 (OpenFoodFacts API Client)
//!
//! 核心合规与架构规范：
//! 1. **强制自定义 User-Agent**：每次 HTTP 请求必须携带显式标识 App 的专属 User-Agent，
//!    严格杜绝 reqwest 默认 UA，防止被 OpenFoodFacts 防爬网关封禁 IP；
//! 2. **客户端直连与防封限速器**：采用滑动时间窗口限速，单个 IP 条码接口限制在 14 次/分以内，
//!    主动避免触发官方 15 次/分硬性阈值；
//! 3. **ODbL 协议合规与署名**：解析并标注数据来源（Open Food Facts / ODbL），提供合法展示元数据；
//! 4. **物理分库与离线优先缓存**：优先检索本地独立缓存库 (`food_cache.db`)，缓存未命中才发起联网，
//!    网络返回后自动落盘缓存，且绝不污染用户主库 (`litebalance.db`)；
//! 5. **未知微量元素零污染**：解析过程中，缺失或为 null 的营养素严格映射为 `None`，不补 `0.0`。

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::Utc;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::client::barcode::{BarcodeError, BarcodeValidator, NormalizedBarcode};
use crate::storage::cache_db::{CacheStorageEngine, CachedFoodRecord};
use crate::storage::models::Nutriments100g;

/// 官方合规默认 User-Agent（格式：应用名/版本 (联系方式或仓库主页)）
pub const DEFAULT_USER_AGENT: &str =
    "LiteBalance-Desktop/1.0 (https://github.com/litebalance; contact@litebalance.org)";

/// OpenFoodFacts v2 标准产品查询接口基地址
pub const OFF_API_BASE_URL: &str = "https://world.openfoodfacts.org/api/v2/product";

/// 远程食品客户端异常类型
#[derive(Error, Debug)]
pub enum RemoteClientError {
    /// 条形码格式或 GS1 校验位错误
    #[error("条形码无效: {0}")]
    InvalidBarcode(#[from] BarcodeError),

    /// 触发客户端本地防封限速
    #[error("触发请求速率限制: 请等待 {retry_after_secs} 秒后重试 (官方限制 15 次/分)")]
    RateLimitExceeded { retry_after_secs: u64 },

    /// 网络传输或 HTTP 状态异常
    #[error("网络传输或 HTTP 请求异常: {0}")]
    Network(#[from] reqwest::Error),

    /// 未在远程数据库中找到该商品
    #[error("OpenFoodFacts 数据库未找到该商品: 条形码 '{0}'")]
    ProductNotFound(String),

    /// 响应数据反序列化异常
    #[error("远程食品数据解析异常: {0}")]
    ParseError(String),

    /// 本地存储/缓存异常
    #[error("本地缓存数据库异常: {0}")]
    Storage(#[from] crate::storage::error::StorageError),
}

/// 客户端本地滑动窗口请求限速器
#[derive(Debug)]
pub struct SlidingWindowRateLimiter {
    /// 允许的最大请求次数
    max_requests: usize,
    /// 时间窗口大小（默认 60 秒）
    window_duration: Duration,
    /// 记录每次请求成功的时刻
    timestamps: Arc<Mutex<VecDeque<Instant>>>,
}

impl SlidingWindowRateLimiter {
    /// 创建限速器（如每 60 秒最多 14 次）
    pub fn new(max_requests: usize, window_duration: Duration) -> Self {
        Self {
            max_requests,
            window_duration,
            timestamps: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// 尝试获取一次请求许可，若超出频率则返回需等待的剩余秒数
    pub fn acquire(&self) -> Result<(), u64> {
        let mut queue = self.timestamps.lock().unwrap();
        let now = Instant::now();

        // 移除窗口时间之外的旧请求
        while let Some(&front) = queue.front() {
            if now.duration_since(front) >= self.window_duration {
                queue.pop_front();
            } else {
                break;
            }
        }

        if queue.len() < self.max_requests {
            queue.push_back(now);
            Ok(())
        } else {
            // 计算最早一次请求何时滑出窗口
            let oldest = *queue.front().unwrap();
            let elapsed = now.duration_since(oldest);
            let wait_duration = self.window_duration.saturating_sub(elapsed);
            let secs = wait_duration.as_secs().max(1);
            Err(secs)
        }
    }
}

/// 远程食品查询统一结果模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteFoodProduct {
    /// 规范化条码信息
    pub barcode: NormalizedBarcode,
    /// 商品名称
    pub food_name: String,
    /// 品牌信息
    pub brand: Option<String>,
    /// 标准份量数值
    pub serving_quantity: Option<f64>,
    /// 标准份量单位
    pub serving_unit: Option<String>,
    /// 清洗后的每 100g 营养素（缺失为 None，绝不填 0）
    pub nutriments: Nutriments100g,
    /// 是否直接命中本地离线缓存（true: 本地直接读取; false: 联网拉取）
    pub from_cache: bool,
    /// 数据来源署名 (ODbL)
    pub attribution: String,
}

/// OpenFoodFacts 远程食品查询客户端
pub struct RemoteFoodClient {
    http_client: Client,
    user_agent: String,
    rate_limiter: SlidingWindowRateLimiter,
}

impl RemoteFoodClient {
    /// 创建远程食品查询客户端实例。
    /// 传入自定义 `custom_user_agent`，若为空则使用合规官方推荐 UA。
    pub fn new(custom_user_agent: Option<&str>) -> Result<Self, RemoteClientError> {
        let ua = custom_user_agent
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .unwrap_or(DEFAULT_USER_AGENT)
            .to_string();

        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&ua)
                .map_err(|e| RemoteClientError::ParseError(format!("无效的 User-Agent 字符串: {}", e)))?,
        );

        let http_client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(10))
            .build()?;

        // 官方限制 15 次/分，设置 14 次/分作为本地安全护栏
        let rate_limiter = SlidingWindowRateLimiter::new(14, Duration::from_secs(60));

        Ok(Self {
            http_client,
            user_agent: ua,
            rate_limiter,
        })
    }

    /// 获取当前客户端配置的 User-Agent
    pub fn user_agent(&self) -> &str {
        &self.user_agent
    }

    /// 核心查询入口：根据条形码查询食品。
    ///
    /// 完整流水线：
    /// 1. 校验并规范化条形码（GS1 模 10 算法，拦截无效码）；
    /// 2. 若传入 `cache_storage`，优先检索独立缓存库 `food_cache.db`（未过期直接返回）；
    /// 3. 本地客户端限速检查；
    /// 4. 携带强制自定义 User-Agent 直连 OpenFoodFacts API 获取 JSON；
    /// 5. 清洗提取数据（保持未知微量为 None）；
    /// 6. 写回 `food_cache.db` 并返回。
    pub fn fetch_by_barcode(
        &self,
        barcode_input: &str,
        cache_storage: Option<&CacheStorageEngine>,
    ) -> Result<RemoteFoodProduct, RemoteClientError> {
        // 1. GS1 模 10 校验与规范化
        let normalized = BarcodeValidator::parse_and_normalize(barcode_input)?;

        // 2. 检查本地独立缓存库
        if let Some(cache) = cache_storage
            && let Some(cached) = cache.get_cached_food(&normalized.standard_code)?
        {
            return Ok(RemoteFoodProduct {
                barcode: normalized,
                food_name: cached.food_name,
                brand: cached.brand,
                serving_quantity: cached.serving_quantity,
                serving_unit: cached.serving_unit,
                nutriments: cached.nutriments,
                from_cache: true,
                attribution: cached.attribution,
            });
        }

        // 3. 客户端防封限速检查
        if let Err(retry_after) = self.rate_limiter.acquire() {
            return Err(RemoteClientError::RateLimitExceeded {
                retry_after_secs: retry_after,
            });
        }

        // 4. 联网直连 OpenFoodFacts API
        let url = format!("{}/{}.json", OFF_API_BASE_URL, normalized.standard_code);
        let resp = self.http_client.get(&url).send()?;

        if !resp.status().is_success() {
            if resp.status().as_u16() == 404 {
                return Err(RemoteClientError::ProductNotFound(normalized.raw));
            }
            return Err(RemoteClientError::ParseError(format!(
                "HTTP 状态异常: {}",
                resp.status()
            )));
        }

        let raw_text = resp.text()?;
        let json_val: serde_json::Value = serde_json::from_str(&raw_text)
            .map_err(|e| RemoteClientError::ParseError(format!("JSON 反序列化失败: {}", e)))?;

        // 校验 product 状态
        let status = json_val.get("status").and_then(|v| v.as_i64()).unwrap_or(0);
        if status != 1 {
            return Err(RemoteClientError::ProductNotFound(normalized.raw));
        }

        let product_obj = json_val
            .get("product")
            .ok_or_else(|| RemoteClientError::ParseError("未找到 product 根对象".into()))?;

        // 5. 数据清洗与提取
        let (food_name, brand, serving_qty, serving_unit, nutriments) = Self::clean_product_data(product_obj);

        let attribution = "Open Food Facts (ODbL)".to_string();

        // 6. 持久化至独立缓存库 (food_cache.db)
        if let Some(cache) = cache_storage {
            let record = CachedFoodRecord {
                barcode: normalized.raw.clone(),
                ean13: normalized.standard_code.clone(),
                food_name: food_name.clone(),
                brand: brand.clone(),
                serving_quantity: serving_qty,
                serving_unit: serving_unit.clone(),
                nutriments: nutriments.clone(),
                raw_json: Some(raw_text),
                cached_at: Utc::now().to_rfc3339(),
                ttl_seconds: 2592000, // 默认 30 天缓存
                attribution: attribution.clone(),
            };
            let _ = cache.save_cached_food(&record);
        }

        Ok(RemoteFoodProduct {
            barcode: normalized,
            food_name,
            brand,
            serving_quantity: serving_qty,
            serving_unit,
            nutriments,
            from_cache: false,
            attribution,
        })
    }

    /// 数据清洗器：从 OpenFoodFacts 的 product JSON 对象中解析标准营养素与份量。
    /// 遵循 0 污染原则：未知或缺失字段一律保留 `None`。
    pub fn clean_product_data(
        p: &serde_json::Value,
    ) -> (String, Option<String>, Option<f64>, Option<String>, Nutriments100g) {
        // 商品名称（多语言回退：product_name_zh -> product_name -> product_name_en）
        let name = p
            .get("product_name_zh")
            .and_then(|v| v.as_str())
            .or_else(|| p.get("product_name").and_then(|v| v.as_str()))
            .or_else(|| p.get("product_name_en").and_then(|v| v.as_str()))
            .unwrap_or("未命名食品")
            .trim()
            .to_string();

        // 品牌
        let brand = p
            .get("brands")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        // 份量提取
        let serving_qty = p
            .get("serving_quantity")
            .and_then(|v| v.as_str().and_then(|s| s.parse::<f64>().ok()).or_else(|| v.as_f64()));

        let serving_unit = if serving_qty.is_some() {
            Some("g".to_string())
        } else {
            None
        };

        // 提取每 100g 营养素
        let mut nutriments = Nutriments100g::simple(0.0, 0.0, 0.0, 0.0);

        if let Some(n) = p.get("nutriments") {
            let extract_f64 = |key: &str| -> Option<f64> {
                n.get(key).and_then(|v| {
                    if let Some(f) = v.as_f64() {
                        Some(f)
                    } else if let Some(s) = v.as_str() {
                        s.parse::<f64>().ok()
                    } else {
                        None
                    }
                })
            };

            // 能量提取 (优先 energy-kcal_100g，若只有 kJ 则除以 4.184)
            if let Some(kcal) = extract_f64("energy-kcal_100g").or_else(|| extract_f64("energy-kcal")) {
                nutriments.energy_kcal_100 = kcal;
            } else if let Some(kj) = extract_f64("energy_100g").or_else(|| extract_f64("energy")) {
                nutriments.energy_kcal_100 = kj / 4.184;
            }

            // 三大宏量营养素
            nutriments.proteins_100 = extract_f64("proteins_100g")
                .or_else(|| extract_f64("proteins"))
                .unwrap_or(0.0);
            nutriments.carbohydrates_100 = extract_f64("carbohydrates_100g")
                .or_else(|| extract_f64("carbohydrates"))
                .unwrap_or(0.0);
            nutriments.fat_100 = extract_f64("fat_100g").or_else(|| extract_f64("fat")).unwrap_or(0.0);

            // 细分脂肪与糖分
            nutriments.sugars_100 = extract_f64("sugars_100g").or_else(|| extract_f64("sugars"));
            nutriments.saturated_fat_100 = extract_f64("saturated-fat_100g").or_else(|| extract_f64("saturated-fat"));
            nutriments.fiber_100 = extract_f64("fiber_100g").or_else(|| extract_f64("fiber"));
            nutriments.monounsaturated_fat_100 = extract_f64("monounsaturated-fat_100g");
            nutriments.polyunsaturated_fat_100 = extract_f64("polyunsaturated-fat_100g");
            nutriments.trans_fat_100 = extract_f64("trans-fat_100g");

            // 矿物质（单位换算：OFF 常见 sodium 为 g，换算为 mg）
            if let Some(s_mg) = extract_f64("sodium_mg_100g") {
                nutriments.sodium_mg_100 = Some(s_mg);
            } else if let Some(s_g) = extract_f64("sodium_100g") {
                nutriments.sodium_mg_100 = Some(s_g * 1000.0);
            }

            // 钾、镁、钙、铁、锌、磷 (OFF 默认常以 g 或 mg 给出，检查相应字段)
            if let Some(k_mg) = extract_f64("potassium_mg_100g") {
                nutriments.potassium_mg_100 = Some(k_mg);
            } else if let Some(k_g) = extract_f64("potassium_100g") {
                nutriments.potassium_mg_100 = Some(k_g * 1000.0);
            }

            if let Some(ca_mg) = extract_f64("calcium_mg_100g") {
                nutriments.calcium_mg_100 = Some(ca_mg);
            } else if let Some(ca_g) = extract_f64("calcium_100g") {
                nutriments.calcium_mg_100 = Some(ca_g * 1000.0);
            }

            if let Some(fe_mg) = extract_f64("iron_mg_100g") {
                nutriments.iron_mg_100 = Some(fe_mg);
            } else if let Some(fe_g) = extract_f64("iron_100g") {
                nutriments.iron_mg_100 = Some(fe_g * 1000.0);
            }

            if let Some(c_mg) = extract_f64("vitamin-c_mg_100g") {
                nutriments.vitamin_c_mg_100 = Some(c_mg);
            } else if let Some(c_g) = extract_f64("vitamin-c_100g") {
                nutriments.vitamin_c_mg_100 = Some(c_g * 1000.0);
            }

            nutriments.magnesium_mg_100 =
                extract_f64("magnesium_mg_100g").or_else(|| extract_f64("magnesium_100g").map(|g| g * 1000.0));
            nutriments.zinc_mg_100 =
                extract_f64("zinc_mg_100g").or_else(|| extract_f64("zinc_100g").map(|g| g * 1000.0));
            nutriments.vitamin_a_ug_100 = extract_f64("vitamin-a_100g").map(|g| g * 1_000_000.0);
            nutriments.vitamin_d_ug_100 = extract_f64("vitamin-d_100g").map(|g| g * 1_000_000.0);
        }

        (name, brand, serving_qty, serving_unit, nutriments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_user_agent_configuration() {
        let client = RemoteFoodClient::new(None).unwrap();
        assert_eq!(client.user_agent(), DEFAULT_USER_AGENT);

        let custom_ua = "NutriTracker-Test/2.0 (test@example.com)";
        let custom_client = RemoteFoodClient::new(Some(custom_ua)).unwrap();
        assert_eq!(custom_client.user_agent(), custom_ua);
    }

    #[test]
    fn test_rate_limiter_allows_and_rejects_bursts() {
        // 限速器：60 秒内最多允许 3 次
        let limiter = SlidingWindowRateLimiter::new(3, Duration::from_secs(60));

        assert!(limiter.acquire().is_ok());
        assert!(limiter.acquire().is_ok());
        assert!(limiter.acquire().is_ok());

        // 第 4 次应当被拦截并返回需等待的剩余秒数
        let err = limiter.acquire();
        assert!(err.is_err());
        let wait_secs = err.unwrap_err();
        assert!((1..=60).contains(&wait_secs));
    }

    #[test]
    fn test_clean_product_data_preserves_none_without_zero_pollution() {
        let sample_json = serde_json::json!({
            "product_name": "Organic Almond Milk",
            "brands": "Silk",
            "serving_quantity": 240,
            "nutriments": {
                "energy-kcal_100g": 13.0,
                "proteins_100g": 0.4,
                "carbohydrates_100g": 0.3,
                "fat_100g": 1.1,
                "calcium_100g": 0.18, // 0.18 g -> 180 mg
                "sodium_100g": 0.07   // 0.07 g -> 70 mg
                // 注意：没有锌、铁、维生素C 等字段
            }
        });

        let (name, brand, serving, unit, nutriments) = RemoteFoodClient::clean_product_data(&sample_json);

        assert_eq!(name, "Organic Almond Milk");
        assert_eq!(brand, Some("Silk".to_string()));
        assert_eq!(serving, Some(240.0));
        assert_eq!(unit, Some("g".to_string()));

        assert_eq!(nutriments.energy_kcal_100, 13.0);
        assert_eq!(nutriments.proteins_100, 0.4);
        assert_eq!(nutriments.carbohydrates_100, 0.3);
        assert_eq!(nutriments.fat_100, 1.1);

        // 验证换算
        assert_eq!(nutriments.calcium_mg_100, Some(180.0));
        assert_eq!(nutriments.sodium_mg_100, Some(70.0));

        // 核心验证：未提供的微量元素必须为 None，决不能被填补为 0.0！
        assert!(nutriments.iron_mg_100.is_none());
        assert!(nutriments.zinc_mg_100.is_none());
        assert!(nutriments.vitamin_c_mg_100.is_none());
    }
}
