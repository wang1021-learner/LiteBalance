//! 条形码解析与 GS1 模 10 校验引擎 (Barcode Validator)
//!
//! 支持标准：
//! - EAN-13（13 位国际标准商品条码）
//! - UPC-A（12 位北美商品条码，自动规范化补零映射为 EAN-13）
//! - EAN-8（8 位短版国际条码）
//!
//! 核心算法：
//! - 遵循 GS1 标准模 10 (Modulo 10) 校验和计算法则；
//! - 严格过滤空格、连字符及非数字伪码，拦截非法请求防刷。

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 条形码解析与校验异常枚举
#[derive(Error, Debug, PartialEq, Eq)]
pub enum BarcodeError {
    /// 包含非数字非法字符
    #[error("条形码包含非数字字符: '{0}'")]
    NonNumeric(String),

    /// 长度不合法（标准仅支持 8 位 EAN-8、12 位 UPC-A 或 13 位 EAN-13）
    #[error("条形码长度非法: 实测 {0} 位（仅支持 8, 12, 13 位）")]
    InvalidLength(usize),

    /// GS1 模 10 校验和不匹配
    #[error("条形码校验位错误: 声明末位为 '{actual}'，GS1 计算期望为 '{expected}'")]
    ChecksumMismatch { actual: char, expected: char },
}

/// 条形码类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BarcodeType {
    /// 13 位 EAN-13 国际条码
    Ean13,
    /// 12 位 UPC-A 北美通用产品代码
    UpcA,
    /// 8 位 EAN-8 紧凑型国际条码
    Ean8,
}

/// 规范化后的条形码数据
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedBarcode {
    /// 用户输入的原始清洗后条码
    pub raw: String,
    /// 规范化后的标准条码（UPC-A 自动前缀补 0 升格为等价 13 位 EAN-13）
    pub standard_code: String,
    /// 识别出的条码标准类型
    pub barcode_type: BarcodeType,
}

/// 条形码校验与规范化器
pub struct BarcodeValidator;

impl BarcodeValidator {
    /// 清洗条码字符串（剔除首尾空白、内部空格以及常见扫码枪产生的短横线分隔符）。
    pub fn sanitize(raw: &str) -> String {
        raw.trim().replace([' ', '-'], "")
    }

    /// 遵循 GS1 官方标准的模 10 (Modulo 10) 校验和算法。
    ///
    /// 规则：
    /// 1. 取除最后一位（校验位）之外的全部数字；
    /// 2. 从最右侧一位（即原倒数第二位）开始向左遍历，权重交替乘以 3 和 1；
    /// 3. 求所有乘积之和 `sum`；
    /// 4. 校验位为 `(10 - (sum % 10)) % 10`。
    pub fn calculate_gs1_checksum(digits_without_check: &str) -> Option<u32> {
        if digits_without_check.is_empty() || !digits_without_check.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }

        let mut sum = 0u32;
        let mut weight = 3u32; // 从校验位左侧第一位起，权重为 3

        for ch in digits_without_check.chars().rev() {
            let digit = ch.to_digit(10)?;
            sum += digit * weight;
            weight = if weight == 3 { 1 } else { 3 };
        }

        Some((10 - (sum % 10)) % 10)
    }

    /// 校验包含最后校验位在内的完整条码是否合法。
    pub fn verify_checksum(full_digits: &str) -> bool {
        if full_digits.len() < 2 {
            return false;
        }
        let (body, check_char) = full_digits.split_at(full_digits.len() - 1);
        let actual_check = match check_char.chars().next().and_then(|c| c.to_digit(10)) {
            Some(d) => d,
            None => return false,
        };

        match Self::calculate_gs1_checksum(body) {
            Some(expected) => expected == actual_check,
            None => false,
        }
    }

    /// 解析、校验并规范化条形码。
    ///
    /// - 12 位 UPC-A：在校验通过后，在前端补充前导 0 规范化为等价 13 位 EAN-13；
    /// - 13 位 EAN-13：校验通过后直接返回；
    /// - 8 位 EAN-8：校验通过后作为 EAN-8 标准码返回。
    pub fn parse_and_normalize(input: &str) -> Result<NormalizedBarcode, BarcodeError> {
        let clean = Self::sanitize(input);

        if !clean.chars().all(|c| c.is_ascii_digit()) {
            return Err(BarcodeError::NonNumeric(clean));
        }

        let len = clean.len();
        let barcode_type = match len {
            13 => BarcodeType::Ean13,
            12 => BarcodeType::UpcA,
            8 => BarcodeType::Ean8,
            other => return Err(BarcodeError::InvalidLength(other)),
        };

        // 校验 GS1 模 10
        let (body, check_slice) = clean.split_at(len - 1);
        let actual_char = check_slice.chars().next().unwrap();
        let expected_digit = Self::calculate_gs1_checksum(body).unwrap();
        let expected_char = char::from_digit(expected_digit, 10).unwrap();

        if actual_char != expected_char {
            return Err(BarcodeError::ChecksumMismatch {
                actual: actual_char,
                expected: expected_char,
            });
        }

        let standard_code = match barcode_type {
            BarcodeType::UpcA => format!("0{}", clean), // UPC-A 规范化升格为 EAN-13
            BarcodeType::Ean13 | BarcodeType::Ean8 => clean.clone(),
        };

        Ok(NormalizedBarcode {
            raw: clean,
            standard_code,
            barcode_type,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_ean13() {
        // 法国依云矿泉水 EAN-13: 3079970000058
        let res = BarcodeValidator::parse_and_normalize("3079970000058").unwrap();
        assert_eq!(res.barcode_type, BarcodeType::Ean13);
        assert_eq!(res.standard_code, "3079970000058");

        // 带空格与短横线的清洗支持: "307-997 000005 8"
        let res_spaced = BarcodeValidator::parse_and_normalize(" 307-997 000005 8 ").unwrap();
        assert_eq!(res_spaced.standard_code, "3079970000058");
    }

    #[test]
    fn test_valid_upca_normalization_to_ean13() {
        // 典型北美 12 位 UPC-A: 737628064502 (Thai Kitchen 椰浆)
        let res = BarcodeValidator::parse_and_normalize("737628064502").unwrap();
        assert_eq!(res.barcode_type, BarcodeType::UpcA);
        assert_eq!(res.raw, "737628064502");
        assert_eq!(res.standard_code, "0737628064502"); // 自动补零升格为 13 位 EAN
    }

    #[test]
    fn test_valid_ean8() {
        // 常见 8 位商品条码: 96385074
        let res = BarcodeValidator::parse_and_normalize("96385074").unwrap();
        assert_eq!(res.barcode_type, BarcodeType::Ean8);
        assert_eq!(res.standard_code, "96385074");
    }

    #[test]
    fn test_checksum_mismatch_rejected() {
        // 将依云末位校验位 8 故意篡改为 4
        let err = BarcodeValidator::parse_and_normalize("3079970000054").unwrap_err();
        assert_eq!(
            err,
            BarcodeError::ChecksumMismatch {
                actual: '4',
                expected: '8',
            }
        );
    }

    #[test]
    fn test_invalid_characters_and_length() {
        // 非数字
        assert!(matches!(
            BarcodeValidator::parse_and_normalize("307997000005A"),
            Err(BarcodeError::NonNumeric(_))
        ));

        // 长度错误（如 10 位）
        assert_eq!(
            BarcodeValidator::parse_and_normalize("1234567890"),
            Err(BarcodeError::InvalidLength(10))
        );
    }
}
