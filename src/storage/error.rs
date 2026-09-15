//! 本地存储与数据库异常定义模块

use thiserror::Error;

/// 本地数据库与持久化层异常类型枚举
#[derive(Error, Debug)]
pub enum StorageError {
    /// SQLite 底层执行异常
    #[error("SQLite 数据库执行错误: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// 目标实体未找到
    #[error("未检索到指定数据实体: {0}")]
    NotFound(String),

    /// 数据序列化 / 反序列化或类型转换异常
    #[error("数据序列化/反序列化类型转换异常: {0}")]
    Conversion(String),

    /// 数据库版本迁移异常
    #[error("数据库结构版本迁移失败: {0}")]
    Migration(String),

    /// 数据库连接互斥锁中毒（此前持锁线程发生 panic）
    ///
    /// 该变体确保锁中毒时以可恢复错误的形式向调用方（含移动端 FFI 边界）返回，
    /// 而非在库内部直接 panic 引发跨语言边界的进程级崩溃。
    #[error("数据库连接锁已中毒（此前持锁线程发生 panic）: {0}")]
    LockPoisoned(String),
}
