package com.litebalance.app.model

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import uniffi.litebalance.FfiUserInput
import java.time.Duration
import java.time.Instant
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.format.DateTimeFormatter

/**
 * 生殖/泌乳状态。
 *
 * 注意：Rust 核心当前仅支持非孕非哺成人 EER。妊娠/哺乳不可在前端用固定加数冒充，
 * 必须禁用能量规划，等待核心接入专用方程。
 */
enum class ReproductiveStatus(val label: String, val note: String) {
    NONE("非孕非哺 (常规)", "可使用 NASEM 2023 / IOM 2005 成人能量方程"),
    PREGNANCY_T1("妊娠早期 (1~12周)", "核心尚未接入妊娠专用 EER，已禁止自动热量计算"),
    PREGNANCY_T2("妊娠中期 (13~28周)", "核心尚未接入妊娠专用 EER，已禁止自动热量计算"),
    PREGNANCY_T3("妊娠晚期 (29~40周)", "核心尚未接入妊娠专用 EER，已禁止自动热量计算"),
    LACTATION("哺乳期 (产后 0~6月)", "核心尚未接入哺乳专用 EER，已禁止自动热量计算"),
    ;

    /** 是否阻断成人能量方程（TDEE / 计划 / 自适应预算） */
    val blocksAdultEnergyEquations: Boolean
        get() = this != NONE
}

enum class HormoneProfile(val label: String) {
    STANDARD_MALE("顺性别雄性激素表型 (Male)"),
    STANDARD_FEMALE("顺性别雌性激素表型 (Female)"),
    TRANS_FEMININE_HRT("跨性别女性 (雌激素 HRT 维持)"),
    TRANS_MASCULINE_HRT("跨性别男性 (雄激素 HRT 维持)"),
    NON_BINARY("非二元 / 未经 HRT"),
}

/**
 * 需求 1: 用户档案资料
 */
data class UserProfileItem(
    val userId: String,
    val name: String,
    val age: Int,
    val heightCm: Double,
    val weightKg: Double,
    val gender: String,          // male / female
    val hormoneProfile: HormoneProfile = HormoneProfile.STANDARD_FEMALE,
    val reproductiveStatus: ReproductiveStatus = ReproductiveStatus.NONE,
    val activityLevel: String = "low_active",
    val isCurrent: Boolean = false,
) {
    fun toFfiUserInput(): FfiUserInput {
        return FfiUserInput(
            userId = userId,
            name = name,
            birthday = "1998-01-01",
            age = age.toUByte(),
            heightCm = heightCm,
            weightKg = weightKg,
            gender = gender,
            activityLevel = activityLevel,
        )
    }

    /** 是否允许调用核心能量规划 API（与 Rust 门禁一致：≥19 且非孕非哺）。 */
    fun allowsEnergyPlanning(): Boolean {
        return age >= 19 && !reproductiveStatus.blocksAdultEnergyEquations
    }

    /** 不可规划时的用户可见原因；可规划时返回 null。 */
    fun energyPlanningBlockReason(): String? {
        return when {
            age < 19 ->
                "当前档案年龄为 ${age} 岁。能量需求 / 体重规划 / 自适应预算仅适用于 ≥19 岁成人，请勿套用成人公式。"
            reproductiveStatus.blocksAdultEnergyEquations ->
                "当前档案为「${reproductiveStatus.label}」。核心尚未实现妊娠/哺乳专用能量方程，已禁用自动热量计算，请咨询临床营养方案。"
            else -> null
        }
    }
}

/**
 * 需求 16: 间歇性断食协议与状态机
 */
enum class FastingProtocol(val label: String, val fastHours: Int, val windowHours: Int) {
    F16_8("16:8 经典断食", 16, 8),
    F18_6("18:6 进阶燃脂", 18, 6),
    F20_4("20:4 勇士断食", 20, 4),
    OMAD("OMAD 一日一餐", 23, 1),
    CIRCADIAN("昼夜节律断食", 14, 10),
}

enum class FastingPhase(val label: String, val description: String) {
    DIGESTION("消化合成期", "血糖及胰岛素处于吸收峰值，能量充足供应"),
    GLYCOGEN_DROP("糖原动员期", "血糖回落，肝糖原开始分解维持基础代谢"),
    FAT_BURNING("酮体燃脂期", "体内胰岛素降至基线，脂肪加速氧化供能"),
    AUTOPHAGY("细胞自噬期", "启动细胞自噬与线粒体清理，抗衰与修复增强"),
}

data class FastingSession(
    val protocol: FastingProtocol = FastingProtocol.F16_8,
    val startTime: Instant? = null,
    val targetEndTime: Instant? = null,
    val isRunning: Boolean = false,
) {
    fun currentPhase(): FastingPhase {
        if (!isRunning || startTime == null) return FastingPhase.DIGESTION
        val hours = Duration.between(startTime, Instant.now()).toHours()
        return when {
            hours < 4 -> FastingPhase.DIGESTION
            hours < 10 -> FastingPhase.GLYCOGEN_DROP
            hours < 16 -> FastingPhase.FAT_BURNING
            else -> FastingPhase.AUTOPHAGY
        }
    }

    fun elapsedMinutes(): Long {
        if (!isRunning || startTime == null) return 0
        return Duration.between(startTime, Instant.now()).toMinutes()
    }

    fun progressPct(): Float {
        if (!isRunning || startTime == null || targetEndTime == null) return 0f
        val total = Duration.between(startTime, targetEndTime).toMinutes().toFloat()
        if (total <= 0f) return 0f
        val elapsed = elapsedMinutes().toFloat()
        return (elapsed / total).coerceIn(0f, 1f)
    }
}

/**
 * 需求 19, 20, 21: 运动与 Pontzer 能量代偿模型
 */
data class ActivityItem(
    val id: String,
    val name: String,
    val category: String, // 力量 / 耐力 / 球类 / 日常
    val met: Double,
    val defaultDurationMin: Int = 30,
)

data class WorkoutLog(
    val id: String,
    val activityName: String,
    val durationMin: Int,
    val nominalKcal: Double,  // 名义总消耗
    val netKcal: Double,      // 真实净消耗 (扣除 Pontzer 代偿挤压)
    val compensationPct: Int, // 代偿阻尼比例 (如 28%)
    val loggedAt: String,
)

/**
 * 需求 23: NASEM DRI 微量营养雷达
 */
data class MicronutrientItem(
    val name: String,
    val amount: Double,
    val unit: String,
    val targetRda: Double,
    val upperLimit: Double?,
    val isCoverageSufficient: Boolean, // 需求 23: 50% 覆盖率防误判
) {
    val ratio: Float get() = if (targetRda > 0) (amount / targetRda).toFloat() else 0f
    val status: String get() = when {
        !isCoverageSufficient -> "数据覆盖不足"
        upperLimit != null && amount > upperLimit -> "超摄入上限"
        ratio >= 0.9f -> "充足达标"
        ratio >= 0.6f -> "基本适中"
        else -> "摄入偏低"
    }
}

/**
 * 需求 12 & 13: 本地自建食物与多原料食谱
 */
data class CustomFoodItem(
    val id: String,
    val name: String,
    val brand: String,
    val basisType: String, // "per_100g" 或 "per_serving"
    val servingSizeG: Double,
    val energyKcal: Double,
    val proteinG: Double,
    val carbsG: Double,
    val fatG: Double,
    val fiberG: Double? = null,
    val sodiumMg: Double? = null,
)

data class RecipeItem(
    val id: String,
    val name: String,
    val description: String,
    val totalWeightG: Double,
    val totalEnergyKcal: Double,
    val totalProteinG: Double,
    val totalCarbsG: Double,
    val totalFatG: Double,
    val ingredients: List<String>,
)

/**
 * 全局应用状态中心 (单例)
 */
object AppStateManager {
    // 1. 多用户档案管理
    val profiles = mutableStateListOf(
        UserProfileItem(
            userId = "user_default",
            name = "轻衡体验者",
            age = 24,
            heightCm = 172.0,
            weightKg = 65.0,
            gender = "female",
            hormoneProfile = HormoneProfile.STANDARD_FEMALE,
            reproductiveStatus = ReproductiveStatus.NONE,
            activityLevel = "low_active",
            isCurrent = true,
        ),
        UserProfileItem(
            userId = "user_athlete",
            name = "力量举训练者",
            age = 29,
            heightCm = 180.0,
            weightKg = 82.0,
            gender = "male",
            hormoneProfile = HormoneProfile.STANDARD_MALE,
            reproductiveStatus = ReproductiveStatus.NONE,
            activityLevel = "active",
            isCurrent = false,
        ),
        // 用于验证：选中后能量规划应被禁用，不得显示虚假加热量
        UserProfileItem(
            userId = "user_pregnancy_demo",
            name = "妊娠演示档案（应禁用热量）",
            age = 28,
            heightCm = 165.0,
            weightKg = 68.0,
            gender = "female",
            hormoneProfile = HormoneProfile.STANDARD_FEMALE,
            reproductiveStatus = ReproductiveStatus.PREGNANCY_T2,
            activityLevel = "low_active",
            isCurrent = false,
        ),
    )

    fun updateCurrentReproductiveStatus(status: ReproductiveStatus) {
        val idx = profiles.indexOfFirst { it.userId == currentUserId }
        if (idx >= 0) {
            profiles[idx] = profiles[idx].copy(reproductiveStatus = status)
        }
    }

    var currentUserId by mutableStateOf("user_default")

    fun currentUser(): UserProfileItem {
        return profiles.find { it.userId == currentUserId } ?: profiles.first()
    }

    fun switchUser(userId: String) {
        currentUserId = userId
    }

    // 2. 跨午夜生理日界线 (需求 14 & 22)
    var dayBoundaryMinutes by mutableStateOf(0u) // 0 表示自然日；240u 表示凌晨 04:00 切换逻辑日

    // 3. 间歇性断食状态机 (需求 16)
    var fastingSession by mutableStateOf(
        FastingSession(
            protocol = FastingProtocol.F16_8,
            startTime = Instant.now().minus(Duration.ofHours(12)),
            targetEndTime = Instant.now().minus(Duration.ofHours(12)).plus(Duration.ofHours(16)),
            isRunning = true,
        ),
    )

    fun startFasting(protocol: FastingProtocol) {
        val now = Instant.now()
        fastingSession = FastingSession(
            protocol = protocol,
            startTime = now,
            targetEndTime = now.plus(Duration.ofHours(protocol.fastHours.toLong())),
            isRunning = true,
        )
    }

    fun stopFasting() {
        fastingSession = fastingSession.copy(isRunning = false)
    }

    // 4. 运动与 Pontzer 能量代偿 (需求 19, 20, 21)
    val standardActivities = listOf(
        ActivityItem("act_walk", "户外快走 (4.8 km/h)", "耐力", 3.5, 45),
        ActivityItem("act_run", "慢跑 (8.0 km/h)", "耐力", 8.3, 30),
        ActivityItem("act_lift", "抗阻力量训练 (中高强度)", "力量", 6.0, 50),
        ActivityItem("act_hiit", "高强度间歇训练 (HIIT)", "力量", 8.5, 25),
        ActivityItem("act_swim", "自由泳巡航", "耐力", 7.0, 40),
        ActivityItem("act_yoga", "哈他瑜伽与拉伸", "日常", 2.5, 60),
    )

    val todayWorkouts = mutableStateListOf(
        WorkoutLog(
            id = "w1",
            activityName = "抗阻力量训练 (中高强度)",
            durationMin = 45,
            nominalKcal = 310.0,
            netKcal = 248.0,
            compensationPct = 20, // 力量训练代偿率较低 (约20%)
            loggedAt = "10:30",
        ),
    )

    fun logWorkout(activity: ActivityItem, durationMin: Int, weightKg: Double) {
        // Herrmann 2024 MET 公式计算名义消耗：MET * 1.05 * weightKg * (durationMin / 60.0)
        val nominal = activity.met * 1.05 * weightKg * (durationMin / 60.0)
        // Pontzer 2026 代偿模型：力量训练代偿 ~20%，高强度耐力代偿 ~28%
        val compPct = if (activity.category == "力量") 20 else 28
        val net = nominal * (1.0 - compPct / 100.0)

        val log = WorkoutLog(
            id = "w_${System.currentTimeMillis()}",
            activityName = activity.name,
            durationMin = durationMin,
            nominalKcal = nominal,
            netKcal = net,
            compensationPct = compPct,
            loggedAt = LocalDateTime.now().format(DateTimeFormatter.ofPattern("HH:mm")),
        )
        todayWorkouts.add(0, log)
    }

    // 5. 本地自建食物与食谱库 (需求 12 & 13)
    val customFoods = mutableStateListOf(
        CustomFoodItem("c1", "生椰减脂蛋白燕麦碗", "自制", "per_serving", 250.0, 320.0, 24.0, 36.0, 8.5, 6.0, 120.0),
        CustomFoodItem("c2", "低钠慢烤黑椒牛肉条", "包装单份", "per_serving", 50.0, 115.0, 19.5, 2.0, 3.2, 0.0, 280.0),
    )

    val customRecipes = mutableStateListOf(
        RecipeItem(
            id = "r1",
            name = "地中海清炖三文鱼时蔬锅",
            description = "高欧米伽-3脂肪酸、去脂护肌均衡食谱",
            totalWeightG = 580.0,
            totalEnergyKcal = 540.0,
            totalProteinG = 42.0,
            totalCarbsG = 22.0,
            totalFatG = 28.0,
            ingredients = listOf("智利三文鱼柳 180g", "初榨橄榄油 10g", "西兰花 200g", "圣女果 150g", "黑胡椒海盐少许"),
        ),
    )

    // 6. 微量元素 DRI 数据 (需求 23)
    val micronutrients = listOf(
        MicronutrientItem("膳食纤维", 26.5, "g", 28.0, null, true),
        MicronutrientItem("钠 (Sodium)", 1650.0, "mg", 1500.0, 2300.0, true),
        MicronutrientItem("钾 (Potassium)", 2800.0, "mg", 3400.0, null, true),
        MicronutrientItem("钙 (Calcium)", 890.0, "mg", 1000.0, 2500.0, true),
        MicronutrientItem("镁 (Magnesium)", 320.0, "mg", 350.0, null, true),
        MicronutrientItem("铁 (Iron)", 14.2, "mg", 18.0, 45.0, true),
        MicronutrientItem("维生素 C", 110.0, "mg", 90.0, 2000.0, true),
        MicronutrientItem("锌 (Zinc)", 4.2, "mg", 8.0, 40.0, false), // 覆盖率不足，触发防误报
    )

    // 7. 外部网络缓存管理 (需求 10 & 29)
    var cachedFoodsCount by mutableStateOf(142)
    var cacheSizeMb by mutableStateOf(1.8)
    var cacheTtlDays by mutableStateOf(30)

    fun clearFoodCache() {
        cachedFoodsCount = 0
        cacheSizeMb = 0.0
    }
}
