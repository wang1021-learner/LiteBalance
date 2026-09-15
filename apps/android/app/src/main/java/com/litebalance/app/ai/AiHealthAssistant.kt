package com.litebalance.app.ai

import com.litebalance.app.model.UserProfileItem
import kotlinx.coroutines.delay

/**
 * 结构化「演示规则」拆解出的食材条目（非真实模型输出）。
 */
data class AiIntakeSuggestion(
    val name: String,
    val amountG: Double,
    val energyKcal: Double,
    val proteinG: Double,
    val carbsG: Double,
    val fatG: Double,
    val note: String,
)

/**
 * 本地关键词 / 模板文案助手。
 *
 * **不是**大模型，也不是 Apple Intelligence。仅用于 UI 演示交互；
 * 营养数值为硬编码示例，不可当作临床或精确计量依据。
 */
object AiHealthAssistant {

    /**
     * 自然语言快记：按关键词返回演示食材列表。
     */
    suspend fun parseNaturalLanguageIntake(prompt: String): List<AiIntakeSuggestion> {
        // 仅模拟网络/思考延迟，便于观察加载态；无模型推理。
        delay(350)

        val clean = prompt.trim()
        val results = mutableListOf<AiIntakeSuggestion>()

        when {
            clean.contains("鸡胸") || clean.contains("鸡肉") -> {
                results.add(
                    AiIntakeSuggestion(
                        name = "香煎低脂鸡胸肉（演示）",
                        amountG = 200.0,
                        energyKcal = 266.0,
                        proteinG = 46.0,
                        carbsG = 0.0,
                        fatG = 6.2,
                        note = "演示条目：关键词「鸡胸/鸡肉」匹配",
                    ),
                )
                if (clean.contains("蛋") || clean.contains("鸡蛋")) {
                    results.add(
                        AiIntakeSuggestion(
                            name = "水煮溏心蛋 (1枚)（演示）",
                            amountG = 50.0,
                            energyKcal = 72.0,
                            proteinG = 6.5,
                            carbsG = 0.6,
                            fatG = 4.8,
                            note = "演示条目：关键词「蛋」匹配",
                        ),
                    )
                }
                if (clean.contains("咖啡") || clean.contains("拿铁")) {
                    results.add(
                        AiIntakeSuggestion(
                            name = "燕麦奶拿铁 (无糖中杯)（演示）",
                            amountG = 350.0,
                            energyKcal = 135.0,
                            proteinG = 3.5,
                            carbsG = 18.0,
                            fatG = 4.5,
                            note = "演示条目：关键词「咖啡/拿铁」匹配",
                        ),
                    )
                } else {
                    results.add(
                        AiIntakeSuggestion(
                            name = "白灼混合时蔬（演示）",
                            amountG = 180.0,
                            energyKcal = 65.0,
                            proteinG = 4.2,
                            carbsG = 11.5,
                            fatG = 0.8,
                            note = "演示条目：默认配菜",
                        ),
                    )
                }
            }

            clean.contains("牛肉") || clean.contains("牛排") -> {
                results.add(
                    AiIntakeSuggestion(
                        name = "嫩煎谷饲眼肉牛排（演示）",
                        amountG = 180.0,
                        energyKcal = 370.0,
                        proteinG = 38.0,
                        carbsG = 0.0,
                        fatG = 23.0,
                        note = "演示条目：关键词「牛肉/牛排」匹配",
                    ),
                )
                results.add(
                    AiIntakeSuggestion(
                        name = "羽衣甘蓝圣女果温沙拉（演示）",
                        amountG = 150.0,
                        energyKcal = 85.0,
                        proteinG = 3.0,
                        carbsG = 9.0,
                        fatG = 4.0,
                        note = "演示条目：默认配菜",
                    ),
                )
            }

            clean.contains("面") || clean.contains("饭") || clean.contains("碳水") -> {
                results.add(
                    AiIntakeSuggestion(
                        name = "慢升糖混合杂粮饭（演示）",
                        amountG = 160.0,
                        energyKcal = 195.0,
                        proteinG = 5.2,
                        carbsG = 41.0,
                        fatG = 1.2,
                        note = "演示条目：关键词「面/饭/碳水」匹配",
                    ),
                )
                results.add(
                    AiIntakeSuggestion(
                        name = "清蒸深海鳕鱼排（演示）",
                        amountG = 120.0,
                        energyKcal = 105.0,
                        proteinG = 23.0,
                        carbsG = 0.0,
                        fatG = 1.1,
                        note = "演示条目：默认配菜",
                    ),
                )
            }

            else -> {
                results.add(
                    AiIntakeSuggestion(
                        name = if (clean.isNotBlank()) "$clean（演示估算）" else "经典平衡高蛋白餐（演示）",
                        amountG = 220.0,
                        energyKcal = 310.0,
                        proteinG = 28.0,
                        carbsG = 24.0,
                        fatG = 9.5,
                        note = "未命中专用关键词，返回通用演示估算",
                    ),
                )
            }
        }

        return results
    }

    /**
     * 基于已算出的预算数字生成模板提示（无模型、无虚构精度声明）。
     */
    fun generateMetabolicInsight(
        user: UserProfileItem,
        currentWeightKg: Double,
        targetWeightKg: Double,
        dailyBudgetKcal: Double,
        baseTdeeKcal: Double,
        isSafetyFloorActive: Boolean,
    ): String {
        val deficit = baseTdeeKcal - dailyBudgetKcal
        val weightDiff = currentWeightKg - targetWeightKg
        val floor = if (user.gender == "male") 1500 else 1200

        return when {
            isSafetyFloorActive ->
                "当前每日预算已触达安全下限（约 ${floor} kcal/天）。这是核心规则提示，不是模型诊断。建议勿再加大赤字，并关注蛋白与恢复。"
            deficit > 750 ->
                "当前相对维持热量约赤字 ${"%.0f".format(deficit)} kcal。演示提示：较大赤字需保证蛋白与训练恢复；请以核心计算结果为准。"
            weightDiff <= 1.0 && weightDiff >= -1.0 ->
                "当前体重 ${"%.1f".format(currentWeightKg)} kg，接近目标 ${"%.1f".format(targetWeightKg)} kg。演示提示：可考虑减小赤字，过渡到维持。"
            else ->
                "今日预算约 ${"%.0f".format(dailyBudgetKcal)} kcal（相对 TDEE ${"%.0f".format(baseTdeeKcal)}）。以上为规则模板文案，不含真实趋势回归或模型拟合。"
        }
    }

    /**
     * 每日能量与饮水的模板提示。
     */
    fun generateDailyReflection(
        netEnergyDifferenceKcal: Double,
        waterPct: Float,
        boundaryShiftActive: Boolean,
    ): String {
        val netStr = "${if (netEnergyDifferenceKcal >= 0) "+" else ""}${"%.0f".format(netEnergyDifferenceKcal)} kcal"
        val waterNote = if (waterPct >= 100f) {
            "今日饮水已达目标。"
        } else {
            "今日饮水尚未达标，可按目标继续补水。"
        }
        val boundaryNote = if (boundaryShiftActive) "已开启跨午夜日界线。" else ""
        return "今日净能量差约 $netStr。$boundaryNote$waterNote（演示提示，非个性化医疗建议。）"
    }
}
