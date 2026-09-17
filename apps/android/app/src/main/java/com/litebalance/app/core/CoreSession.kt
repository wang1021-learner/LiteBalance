package com.litebalance.app.core

import android.content.Context
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withContext
import uniffi.litebalance.FfiDailySummary
import uniffi.litebalance.FfiEnergyBalance
import uniffi.litebalance.FfiEnergyComparison
import uniffi.litebalance.FfiFastingStatus
import uniffi.litebalance.FfiFoodItem
import uniffi.litebalance.FfiGoalInput
import uniffi.litebalance.FfiIntakeLog
import uniffi.litebalance.FfiUserInput
import uniffi.litebalance.FfiUserProfile
import uniffi.litebalance.FfiWaterSummary
import uniffi.litebalance.FfiWeightPoint
import uniffi.litebalance.LiteBalanceSession
import java.io.File
import java.time.LocalDate

/**
 * 轻衡核心计算引擎 UniFFI 移动端桥接门面单例。
 *
 * 保证所有底层 SQLite I/O 与数值密集型微积分求解均严格在 [Dispatchers.IO] 线程池中执行，
 * 杜绝阻塞 Android 主 UI 渲染循环。
 */
object CoreSession {
    private const val TAG = "LiteBalanceCore"

    private var session: LiteBalanceSession? = null

    private val _isReady = MutableStateFlow(false)
    val isReady: StateFlow<Boolean> = _isReady.asStateFlow()

    private val _currentUserId = MutableStateFlow("user_default_01")
    val currentUserId: StateFlow<String> = _currentUserId.asStateFlow()

    private val _currentUserProfile = MutableStateFlow<FfiUserProfile?>(null)
    val currentUserProfile: StateFlow<FfiUserProfile?> = _currentUserProfile.asStateFlow()

    private val _lastError = MutableStateFlow<String?>(null)
    val lastError: StateFlow<String?> = _lastError.asStateFlow()

    /**
     * 在后台线程安全初始化沙盒数据库并拉起 UniFFI Session。
     */
    suspend fun initialize(context: Context): Boolean = withContext(Dispatchers.IO) {
        if (session != null) return@withContext true

        try {
            val dbFile = File(context.filesDir, "litebalance.db").absolutePath
            val cacheFile = File(context.filesDir, "food_cache.db").absolutePath

            Log.i(TAG, "初始化轻衡沙箱会话: db=$dbFile, cache=$cacheFile")
            val s = LiteBalanceSession.open(dbFile, cacheFile)
            session = s

            // 检查或自动注入默认用户 (成年健康女性，28岁，165cm，60kg，轻度活动)
            ensureDefaultUser(s)
            _isReady.value = true
            true
        } catch (t: Throwable) {
            val msg = "初始化底层核心失败: ${t.message}"
            Log.e(TAG, msg, t)
            _lastError.value = msg
            _isReady.value = false
            false
        }
    }

    private fun ensureDefaultUser(s: LiteBalanceSession) {
        val uid = _currentUserId.value
        val existing = s.getUser(uid)
        if (existing == null) {
            val defaultInput = FfiUserInput(
                userId = uid,
                name = "默认档案",
                birthday = "1998-05-15",
                age = 28u.toUByte(),
                heightCm = 165.0,
                weightKg = 60.0,
                gender = "female",
                activityLevel = "low_active"
            )
            s.upsertUser(defaultInput)
            _currentUserProfile.value = s.getUser(uid)
        } else {
            _currentUserProfile.value = existing
        }
    }

    /**
     * 获取指定日期的日汇总 (摄入宏量、总热量、打卡笔数)
     */
    suspend fun getDailySummary(date: String = LocalDate.now().toString()): Result<FfiDailySummary> =
        withContext(Dispatchers.IO) {
            runCatching {
                val s = session ?: error("CoreSession 尚未就绪")
                s.dailySummary(_currentUserId.value, date, 0u)
            }
        }

    /**
     * 获取指定日期的能量天平结算 (摄入 - 维持TDEE - 运动净消耗 = 净赤字/盈余)
     */
    suspend fun getEnergyBalance(date: String = LocalDate.now().toString()): Result<FfiEnergyBalance> =
        withContext(Dispatchers.IO) {
            runCatching {
                val s = session ?: error("CoreSession 尚未就绪")
                val profile = _currentUserProfile.value ?: error("档案尚未就绪")
                val input = profileToInput(profile)
                s.energyBalance(input, date)
            }
        }

    /**
     * 计算 NASEM 2023 临床维持能耗对比
     */
    suspend fun computeTdee(input: FfiUserInput): Result<FfiEnergyComparison> =
        withContext(Dispatchers.IO) {
            runCatching {
                val s = session ?: error("CoreSession 尚未就绪")
                s.computeTdee(input)
            }
        }

    /**
     * 获取 NASEM 推荐饮水目标 (毫升)
     */
    suspend fun getWaterTarget(): Result<UInt> =
        withContext(Dispatchers.IO) {
            runCatching {
                val s = session ?: error("CoreSession 尚未就绪")
                val profile = _currentUserProfile.value ?: error("档案尚未就绪")
                s.waterTarget(profileToInput(profile))
            }
        }

    /**
     * FTS5 离线食物检索
     */
    suspend fun searchFoods(query: String, limit: UInt = 20u): Result<List<FfiFoodItem>> =
        withContext(Dispatchers.IO) {
            runCatching {
                val s = session ?: error("CoreSession 尚未就绪")
                s.searchFoods(query, limit)
            }
        }

    /**
     * 饮食打卡
     */
    suspend fun logFood(
        foodId: String,
        amountG: Double,
        mealType: String,
        consumedAt: String
    ): Result<FfiIntakeLog> = withContext(Dispatchers.IO) {
        runCatching {
            val s = session ?: error("CoreSession 尚未就绪")
            s.logFood(_currentUserId.value, foodId, amountG, mealType, consumedAt)
        }
    }

    /**
     * 记录饮水 (毫升)
     */
    suspend fun logWater(amountMl: UInt, loggedAt: String): Result<String> =
        withContext(Dispatchers.IO) {
            runCatching {
                val s = session ?: error("CoreSession 尚未就绪")
                s.logWater(_currentUserId.value, amountMl, loggedAt)
            }
        }

    /**
     * 获取当日饮水目标与进度 (来自 SQLite 与 NASEM 公式)
     */
    suspend fun getDailyWater(date: String = LocalDate.now().toString()): Result<FfiWaterSummary> =
        withContext(Dispatchers.IO) {
            runCatching {
                val s = session ?: error("CoreSession 尚未就绪")
                val profile = _currentUserProfile.value?.let { profileToInput(it) }
                s.dailyWater(_currentUserId.value, date, 0u, profile)
            }
        }

    /**
     * 获取间歇性断食当前状态
     */
    suspend fun getFastingStatus(): Result<FfiFastingStatus> =
        withContext(Dispatchers.IO) {
            runCatching {
                val s = session ?: error("CoreSession 尚未就绪")
                s.fastingStatus(_currentUserId.value)
            }
        }

    /**
     * 开始断食会话
     */
    suspend fun startFasting(
        protocol: String = "16:8",
        startedAt: String = java.time.Instant.now().toString()
    ): Result<String> = withContext(Dispatchers.IO) {
        runCatching {
            val s = session ?: error("CoreSession 尚未就绪")
            s.startFasting(_currentUserId.value, protocol, startedAt)
        }
    }

    /**
     * 完成断食会话
     */
    suspend fun completeFasting(
        sessionId: String,
        completedAt: String = java.time.Instant.now().toString()
    ): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val s = session ?: error("CoreSession 尚未就绪")
            s.completeFasting(sessionId, completedAt)
        }
    }

    /**
     * 记录实测体重与体脂
     */
    suspend fun logWeight(weightKg: Double, bodyFatPct: Double?, loggedAt: String): Result<String> =
        withContext(Dispatchers.IO) {
            runCatching {
                val s = session ?: error("CoreSession 尚未就绪")
                s.logWeight(_currentUserId.value, weightKg, bodyFatPct, loggedAt)
            }
        }

    /**
     * 获取最近的体重轨迹点
     */
    suspend fun getWeightHistory(limit: UInt = 30u): Result<List<FfiWeightPoint>> =
        withContext(Dispatchers.IO) {
            runCatching {
                val s = session ?: error("CoreSession 尚未就绪")
                s.weightHistory(_currentUserId.value, limit)
            }
        }

    /**
     * 更新用户档案并刷新状态
     */
    suspend fun updateUserProfile(input: FfiUserInput): Result<Unit> =
        withContext(Dispatchers.IO) {
            runCatching {
                val s = session ?: error("CoreSession 尚未就绪")
                s.upsertUser(input)
                _currentUserProfile.value = s.getUser(input.userId)
            }
        }

    /**
     * 将读出的 FfiUserProfile 转换为输入结构 FfiUserInput
     */
    fun profileToInput(p: FfiUserProfile): FfiUserInput {
        return FfiUserInput(
            userId = p.userId,
            name = p.name,
            birthday = p.birthday,
            age = p.age,
            heightCm = p.heightCm,
            weightKg = p.weightKg,
            gender = p.gender,
            activityLevel = p.activityLevel
        )
    }
}
