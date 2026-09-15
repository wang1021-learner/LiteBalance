package com.litebalance.app.ui

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableDoubleStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.litebalance.app.CoreSession
import com.litebalance.app.ai.AiHealthAssistant
import com.litebalance.app.model.AppStateManager
import com.litebalance.app.ui.components.AiInsightCard
import com.litebalance.app.ui.components.AppleButton
import com.litebalance.app.ui.components.AppleCard
import com.litebalance.app.ui.components.AppleMetricItem
import com.litebalance.app.ui.components.AppleSegmentedControl
import com.litebalance.app.ui.components.AppleTextField
import com.litebalance.app.ui.components.CapsuleBadge
import com.litebalance.app.ui.theme.AppleColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.litebalance.FfiBudgetResult
import uniffi.litebalance.FfiEnergyComparison
import uniffi.litebalance.FfiGoalInput
import uniffi.litebalance.FfiPlanResult

/**
 * 能量与体重规划中心（Kevin Hall / 自适应预算；孕哺与未成年禁用）
 */
@Composable
fun PlanScreen() {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val scrollState = rememberScrollState()
    val currentUser = AppStateManager.currentUser()

    var selectedTab by remember { mutableIntStateOf(0) }
    val tabs = listOf("自适应预算", "Kevin Hall 规划", "饮食宏量协议")

    // 状态缓存 (真实 Rust 计算结果)
    var budgetResult by remember { mutableStateOf<FfiBudgetResult?>(null) }
    var planResult by remember { mutableStateOf<FfiPlanResult?>(null) }
    var tdeeComparison by remember { mutableStateOf<FfiEnergyComparison?>(null) }
    var isCalculating by remember { mutableStateOf(false) }

    // 表单状态
    var targetWeightKg by remember { mutableStateOf("60.0") }
    var weeklyRateKg by remember { mutableStateOf("-0.50") }
    var planWeeks by remember { mutableStateOf("12") }
    var taperEnabled by remember { mutableStateOf(true) }
    var adaptiveEnabled by remember { mutableStateOf(true) }
    var blockReason by remember { mutableStateOf<String?>(null) }

    fun refreshAllCalculations() {
        scope.launch {
            isCalculating = true
            val blocked = currentUser.energyPlanningBlockReason()
            if (blocked != null) {
                blockReason = blocked
                budgetResult = null
                tdeeComparison = null
                planResult = null
                isCalculating = false
                return@launch
            }
            blockReason = null
            runCatching {
                withContext(Dispatchers.IO) {
                    val session = CoreSession.get(context)
                    val input = currentUser.toFfiUserInput()
                    session.upsertUser(input)

                    // 1. 设置体态目标
                    session.setGoal(
                        input.userId,
                        FfiGoalInput(
                            kind = if ((targetWeightKg.toDoubleOrNull() ?: 60.0) < currentUser.weightKg) "lose" else "gain",
                            targetWeightKg = targetWeightKg.toDoubleOrNull() ?: 60.0,
                            weeklyRateKg = weeklyRateKg.toDoubleOrNull() ?: -0.5,
                            taperEnabled = taperEnabled,
                            adaptiveEnabled = adaptiveEnabled,
                            manualCalorieOffset = 0.0,
                        ),
                    )

                    // 2. 自适应预算
                    val budget = session.computeBudget(input)
                    // 3. TDEE 对比
                    val tdee = session.computeTdee(input)
                    // 4. Kevin Hall 动态常微分方程仿真
                    val plan = session.computePlan(
                        input,
                        targetWeightKg.toDoubleOrNull() ?: 60.0,
                        (planWeeks.toUIntOrNull() ?: 12u).coerceAtLeast(1u),
                    )

                    Triple(budget, tdee, plan)
                }
            }.onSuccess { (budget, tdee, plan) ->
                budgetResult = budget
                tdeeComparison = tdee
                planResult = plan
            }.onFailure {
                blockReason = it.message
                budgetResult = null
                tdeeComparison = null
                planResult = null
            }
            isCalculating = false
        }
    }

    LaunchedEffect(currentUser.userId) {
        refreshAllCalculations()
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background)
            .verticalScroll(scrollState)
            .padding(horizontal = 20.dp, vertical = 16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        // 1. 顶部标题
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.Bottom,
        ) {
            Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(
                    text = "METABOLIC PLANNING",
                    style = MaterialTheme.typography.labelMedium.copy(
                        fontWeight = FontWeight.SemiBold,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        letterSpacing = 0.5.sp,
                    ),
                )
                Text(
                    text = "代谢规划",
                    style = MaterialTheme.typography.displayLarge.copy(
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onSurface,
                    ),
                )
            }
            CapsuleBadge(text = "NIH & NASEM", color = AppleColors.IrisPurple)
        }

        blockReason?.let { reason ->
            AppleCard {
                CapsuleBadge(text = "已禁用热量计算", color = AppleColors.VitalityCoral)
                Text(
                    text = reason,
                    style = MaterialTheme.typography.bodyMedium.copy(
                        fontWeight = FontWeight.SemiBold,
                        color = MaterialTheme.colorScheme.onSurface,
                    ),
                )
                Text(
                    text = "饮食打卡、饮水、体重记录仍可使用；能量方程需待核心支持该生命阶段后再开放。",
                    style = MaterialTheme.typography.labelMedium.copy(
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    ),
                )
            }
        }

        // 2. 分段选择器
        AppleSegmentedControl(
            items = tabs,
            selectedIndex = selectedTab,
            onSelect = { selectedTab = it },
        )

        when (selectedTab) {
            0 -> AdaptiveBudgetTab(
                budget = budgetResult,
                tdee = tdeeComparison,
                targetWeight = targetWeightKg,
                onTargetWeightChange = { targetWeightKg = it },
                weeklyRate = weeklyRateKg,
                onWeeklyRateChange = { weeklyRateKg = it },
                taper = taperEnabled,
                onTaperChange = { taperEnabled = it },
                adaptive = adaptiveEnabled,
                onAdaptiveChange = { adaptiveEnabled = it },
                onRefresh = { refreshAllCalculations() },
                isCalculating = isCalculating,
            )
            1 -> KevinHallPlanTab(
                plan = planResult,
                targetWeight = targetWeightKg,
                onTargetWeightChange = { targetWeightKg = it },
                weeks = planWeeks,
                onWeeksChange = { planWeeks = it },
                onRefresh = { refreshAllCalculations() },
                isCalculating = isCalculating,
            )
            2 -> MacroProtocolsTab()
        }
    }
}

/**
 * 分区 1: 自适应每日热量预算与安全底线守护 (真实 API)
 */
@Composable
private fun AdaptiveBudgetTab(
    budget: FfiBudgetResult?,
    tdee: FfiEnergyComparison?,
    targetWeight: String,
    onTargetWeightChange: (String) -> Unit,
    weeklyRate: String,
    onWeeklyRateChange: (String) -> Unit,
    taper: Boolean,
    onTaperChange: (Boolean) -> Unit,
    adaptive: Boolean,
    onAdaptiveChange: (Boolean) -> Unit,
    onRefresh: () -> Unit,
    isCalculating: Boolean,
) {
    val currentUser = AppStateManager.currentUser()

    // 核心卡片：预算大字与安全底线指示
    if (budget != null) {
        AppleCard {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.Top,
            ) {
                Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text(
                        text = "每日自适应推荐热量",
                        style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold),
                    )
                    Text(
                        text = "基于 NASEM 维持基准与 OLS 实测体重斜率反馈",
                        style = MaterialTheme.typography.labelSmall.copy(color = MaterialTheme.colorScheme.onSurfaceVariant),
                    )
                }
                CapsuleBadge(
                    text = if (budget.safetyFloorTriggered) "安全红线已触发" else "良性赤字",
                    color = if (budget.safetyFloorTriggered) AppleColors.VitalityCoral else AppleColors.MintGreen,
                )
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.Bottom,
            ) {
                AppleMetricItem(
                    label = "推荐每日预算",
                    value = "%.0f".format(budget.dailyBudgetKcal),
                    unit = "kcal",
                    accentColor = AppleColors.VitalityCoral,
                )
                AppleMetricItem(
                    label = "基准维持 TDEE",
                    value = "%.0f".format(budget.baseTdeeKcal),
                    unit = "kcal",
                    accentColor = MaterialTheme.colorScheme.onSurface,
                )
                AppleMetricItem(
                    label = "内分泌安全底线",
                    value = "%.0f".format(budget.safetyFloorKcal),
                    unit = "kcal",
                    accentColor = AppleColors.AmberGold,
                )
            }

            // 推荐宏量分布
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(14.dp))
                    .background(MaterialTheme.colorScheme.surfaceVariant)
                    .padding(12.dp),
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Text(
                        text = "推荐配比：P ${"%.0f".format(budget.recommendedProteinG)}g · C ${"%.0f".format(budget.recommendedCarbsG)}g · F ${"%.0f".format(budget.recommendedFatG)}g",
                        style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.SemiBold),
                    )
                }
            }
        }
    }

    // 规则模板提示（非真实模型推理）
    if (budget != null) {
        val tip = AiHealthAssistant.generateMetabolicInsight(
            user = currentUser,
            currentWeightKg = currentUser.weightKg,
            targetWeightKg = targetWeight.toDoubleOrNull() ?: 60.0,
            dailyBudgetKcal = budget.dailyBudgetKcal,
            baseTdeeKcal = budget.baseTdeeKcal,
            isSafetyFloorActive = budget.safetyFloorTriggered,
        )
        AiInsightCard(
            title = "预算提示（演示规则）",
            insight = tip,
            badgeText = "演示规则",
        )
    }

    // 目标与算法设置卡片
    AppleCard {
        Text(
            text = "体态目标与自适应参数",
            style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold),
        )

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Box(modifier = Modifier.weight(1f)) {
                AppleTextField(
                    value = targetWeight,
                    onValueChange = onTargetWeightChange,
                    label = "目标体重 (kg)",
                    trailingText = "kg",
                )
            }
            Box(modifier = Modifier.weight(1f)) {
                AppleTextField(
                    value = weeklyRate,
                    onValueChange = onWeeklyRateChange,
                    label = "每周速率 (kg/周)",
                    trailingText = "kg",
                )
            }
        }

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column {
                Text("临界平稳着陆 (Taper Landing)", style = MaterialTheme.typography.bodyMedium.copy(fontWeight = FontWeight.Medium))
                Text("接近目标 1.5kg 时自动递减赤字防反弹", style = MaterialTheme.typography.labelSmall.copy(color = MaterialTheme.colorScheme.onSurfaceVariant))
            }
            Switch(
                checked = taper,
                onCheckedChange = onTaperChange,
                colors = SwitchDefaults.colors(checkedThumbColor = Color.White, checkedTrackColor = AppleColors.MintGreen),
            )
        }

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column {
                Text("OLS 斜率自适应纠偏 (Adaptive)", style = MaterialTheme.typography.bodyMedium.copy(fontWeight = FontWeight.Medium))
                Text("依据实测体重偏离度自动调节下周热量", style = MaterialTheme.typography.labelSmall.copy(color = MaterialTheme.colorScheme.onSurfaceVariant))
            }
            Switch(
                checked = adaptive,
                onCheckedChange = onAdaptiveChange,
                colors = SwitchDefaults.colors(checkedThumbColor = Color.White, checkedTrackColor = AppleColors.MintGreen),
            )
        }

        AppleButton(
            text = if (isCalculating) "正在重新求解…" else "更新目标并重新计算预算",
            onClick = onRefresh,
            modifier = Modifier.fillMaxWidth(),
        )
    }

    // TDEE 模型对比卡片 (需求 4)
    if (tdee != null) {
        AppleCard {
            Text(
                text = "TDEE 临床模型比对 (NASEM 2023 vs IOM 2005)",
                style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold),
            )
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                AppleMetricItem(
                    label = "NASEM 2023 (新标准)",
                    value = "%.0f".format(tdee.nasem2023Kcal),
                    unit = "kcal",
                    sublabel = "8分组多项式矩阵",
                    accentColor = AppleColors.MintGreen,
                )
                AppleMetricItem(
                    label = "IOM 2005 (旧标准)",
                    value = "%.0f".format(tdee.iom2005Kcal),
                    unit = "kcal",
                    sublabel = "传统线性方程",
                    accentColor = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Text(
                text = "两代医学标准差值：${"%.0f".format(tdee.differenceKcal)} kcal。NASEM 2023 对现代低活动量人群准确度提升 18%。",
                style = MaterialTheme.typography.labelSmall.copy(color = MaterialTheme.colorScheme.onSurfaceVariant),
            )
        }
    }
}

/**
 * 分区 2: NIH Kevin Hall 动态常微分方程仿真规划 (真实 API)
 */
@Composable
private fun KevinHallPlanTab(
    plan: FfiPlanResult?,
    targetWeight: String,
    onTargetWeightChange: (String) -> Unit,
    weeks: String,
    onWeeksChange: (String) -> Unit,
    onRefresh: () -> Unit,
    isCalculating: Boolean,
) {
    Column(verticalArrangement = Arrangement.spacedBy(14.dp)) {
        AppleCard {
            Text(
                text = "NIH Kevin Hall 动态体重演化模型",
                style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold),
            )
            Text(
                text = "美国国家卫生研究院 Lancet 2011 临床级动态常微分方程（ODE）。拒绝简单粗暴的 7700 大卡经验公式，全面仿真代谢适应、瘦体重流失阻尼与平台期。",
                style = MaterialTheme.typography.bodyMedium.copy(color = MaterialTheme.colorScheme.onSurfaceVariant),
            )

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                Box(modifier = Modifier.weight(1f)) {
                    AppleTextField(value = targetWeight, onValueChange = onTargetWeightChange, label = "目标体重 (kg)")
                }
                Box(modifier = Modifier.weight(1f)) {
                    AppleTextField(value = weeks, onValueChange = onWeeksChange, label = "规划周期 (周)")
                }
            }

            AppleButton(
                text = if (isCalculating) "微分方程求解中…" else "启动 Kevin Hall 动态仿真",
                onClick = onRefresh,
                modifier = Modifier.fillMaxWidth(),
            )
        }

        if (plan != null) {
            AppleCard(backgroundColor = AppleColors.MintGreenSoft.copy(alpha = 0.4f)) {
                Text(
                    text = "微分仿真预测结果",
                    style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold),
                )

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    AppleMetricItem(
                        label = "期末预测体重",
                        value = "%.2f".format(plan.finalWeightKg),
                        unit = "kg",
                        accentColor = AppleColors.VitalityCoral,
                    )
                    AppleMetricItem(
                        label = "脂肪量变化",
                        value = "%.2f".format(plan.fatMassChangeKg),
                        unit = "kg",
                        accentColor = AppleColors.AmberGold,
                    )
                    AppleMetricItem(
                        label = "瘦体重变化",
                        value = "%.2f".format(plan.fatFreeMassChangeKg),
                        unit = "kg",
                        accentColor = AppleColors.MintGreen,
                    )
                }

                Text(
                    text = "每日动态推荐摄入：${"%.0f".format(plan.recommendedIntakeKcal)} kcal\n" +
                        "推荐宏量配置：蛋白质 ${"%.0f".format(plan.proteinG)}g · 碳水 ${"%.0f".format(plan.carbsG)}g · 脂肪 ${"%.0f".format(plan.fatG)}g",
                    style = MaterialTheme.typography.bodyMedium.copy(fontWeight = FontWeight.SemiBold),
                )
            }
        }
    }
}

/**
 * 分区 3: 循证宏量营养素分配与饮食协议 (需求 6 Demo)
 */
@Composable
private fun MacroProtocolsTab() {
    val protocols = listOf(
        Triple("高蛋白均衡协议 (Balance High-Protein)", "P 30% / C 45% / F 25%", "最适合大众减脂增肌，去脂体重保护与高饱腹感"),
        Triple("低碳温和生酮 (Mild Ketogenic)", "P 25% / C 10% / F 65%", "严格控糖与酮体供能，快速消除水肿并降低胰岛素抵抗"),
        Triple("碳水循环协议 (Carb Cycling)", "高碳日 320g / 低碳日 80g", "高低碳水交替，避免瘦素抵抗，打破顽固平台期"),
        Triple("地中海长寿饮食 (Mediterranean)", "P 20% / C 50% / F 30%", "富含单不饱和脂肪酸与全谷多酚，心血管友好"),
    )

    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
        protocols.forEach { (name, ratio, desc) ->
            AppleCard {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Text(text = name, style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold))
                    CapsuleBadge(text = ratio, color = AppleColors.CalmIndigo)
                }
                Text(text = desc, style = MaterialTheme.typography.bodyMedium.copy(color = MaterialTheme.colorScheme.onSurfaceVariant))
            }
        }
    }
}
