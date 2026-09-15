package com.litebalance.app.ui

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Add
import androidx.compose.material.icons.rounded.Bedtime
import androidx.compose.material.icons.rounded.FitnessCenter
import androidx.compose.material.icons.rounded.LocalDrink
import androidx.compose.material.icons.rounded.Refresh
import androidx.compose.material.icons.rounded.Restaurant
import androidx.compose.material.icons.rounded.WbSunny
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
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
import com.litebalance.app.ui.components.AppleActivityRings
import com.litebalance.app.ui.components.AppleCard
import com.litebalance.app.ui.components.AppleMetricItem
import com.litebalance.app.ui.components.CapsuleBadge
import com.litebalance.app.ui.theme.AppleColors
import com.litebalance.app.ui.theme.LiquidGlassTokens
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.format.DateTimeFormatter
import java.util.Locale

/**
 * 今日仪表盘 (Apple Health 今日闭环与能量天平)
 */
@Composable
fun DashboardScreen() {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val scrollState = rememberScrollState()

    val currentUser = AppStateManager.currentUser()

    // 核心数据状态 (真实 Rust 会话获取)
    var totalIntakeKcal by remember { mutableDoubleStateOf(0.0) }
    var proteinG by remember { mutableDoubleStateOf(0.0) }
    var carbsG by remember { mutableDoubleStateOf(0.0) }
    var fatG by remember { mutableDoubleStateOf(0.0) }
    var itemsCount by remember { mutableIntStateOf(0) }
    var baseTdeeKcal by remember { mutableDoubleStateOf(2150.0) }
    var waterCurrentMl by remember { mutableIntStateOf(1250) }
    var waterGoalMl by remember { mutableIntStateOf(2500) }
    var waterProgressPct by remember { mutableDoubleStateOf(50.0) }
    var statusMessage by remember { mutableStateOf("正在同步代谢核心…") }

    // 运动真实/模拟数据（净运动消耗）
    val netExerciseKcal = AppStateManager.todayWorkouts.sumOf { it.netKcal }
    val nominalExerciseKcal = AppStateManager.todayWorkouts.sumOf { it.nominalKcal }

    // 净能量差 = 摄入 - 基础维持TDEE - 运动净消耗
    val netEnergyDiff = totalIntakeKcal - baseTdeeKcal - netExerciseKcal

    fun loadTodayData() {
        scope.launch {
            statusMessage = "更新中…"
            val energyBlocked = currentUser.energyPlanningBlockReason()
            runCatching {
                withContext(Dispatchers.IO) {
                    val session = CoreSession.get(context)
                    val input = currentUser.toFfiUserInput()
                    session.upsertUser(input)

                    val today = LocalDate.now().toString()
                    val summary = session.dailySummary(input.userId, today, AppStateManager.dayBoundaryMinutes)
                    val water = session.dailyWater(input.userId, today, 0u, input)
                    val tdeeKcal = if (energyBlocked == null) {
                        session.computeTdee(input).nasem2023Kcal
                    } else {
                        null
                    }
                    Triple(summary, tdeeKcal, water)
                }
            }.onSuccess { (summary, tdeeKcal, water) ->
                totalIntakeKcal = summary.totalEnergyKcal
                proteinG = summary.totalProteinG
                carbsG = summary.totalCarbsG
                fatG = summary.totalFatG
                itemsCount = summary.itemsCount.toInt()
                if (tdeeKcal != null) {
                    baseTdeeKcal = tdeeKcal
                }
                waterCurrentMl = water.totalConsumedMl.toInt()
                waterGoalMl = water.goalMl.toInt()
                waterProgressPct = water.progressPct
                statusMessage = if (energyBlocked != null) {
                    "饮食/饮水已同步；能量规划已禁用（${currentUser.reproductiveStatus.label}）"
                } else {
                    "已同步 ${LocalDateTime.now().format(DateTimeFormatter.ofPattern("HH:mm:ss"))}"
                }
            }.onFailure {
                statusMessage = "提示：${it.message ?: "未连接原生核心，已载入离线基准"}"
            }
        }
    }

    LaunchedEffect(currentUser.userId, AppStateManager.dayBoundaryMinutes) {
        loadTodayData()
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background)
            .verticalScroll(scrollState)
            .padding(horizontal = 20.dp, vertical = 16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        // 1. 顶部 Apple Health 风格大标题与日期栏
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.Bottom,
        ) {
            Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(
                    text = LocalDate.now().format(DateTimeFormatter.ofPattern("M月d日 EEEE", Locale.CHINESE)).uppercase(),
                    style = MaterialTheme.typography.labelMedium.copy(
                        fontWeight = FontWeight.SemiBold,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        letterSpacing = 0.5.sp,
                    ),
                )
                Text(
                    text = "今日健康",
                    style = MaterialTheme.typography.displayLarge.copy(
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onSurface,
                    ),
                )
            }

            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                CapsuleBadge(
                    text = currentUser.name,
                    color = AppleColors.MintGreen,
                    backgroundColor = AppleColors.MintGreenSoft,
                )
                IconButton(
                    onClick = { loadTodayData() },
                    modifier = Modifier
                        .size(36.dp)
                        .clip(CircleShape)
                        .background(MaterialTheme.colorScheme.surfaceVariant),
                ) {
                    Icon(
                        imageVector = Icons.Rounded.Refresh,
                        contentDescription = "刷新",
                        tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.size(18.dp),
                    )
                }
            }
        }

        // 2. 苹果标志性三色活动闭环主卡片 (Activity Rings & Energy Balance)
        AppleCard(customGlowBrush = LiquidGlassTokens.CoralLiquidGlow) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Column(
                    modifier = Modifier.weight(1f),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Text(
                        text = "今日代谢闭环",
                        style = MaterialTheme.typography.titleLarge.copy(fontWeight = FontWeight.Bold),
                    )

                    AppleMetricItem(
                        label = "饮食摄入",
                        value = "%.0f".format(totalIntakeKcal),
                        unit = "kcal",
                        sublabel = "基准维持 %.0f kcal".format(baseTdeeKcal),
                        accentColor = AppleColors.VitalityCoral,
                    )

                    AppleMetricItem(
                        label = "运动净计入",
                        value = "%.0f".format(netExerciseKcal),
                        unit = "kcal",
                        sublabel = if (nominalExerciseKcal > netExerciseKcal) {
                            "名义 %.0f (代偿 -%.0f)".format(nominalExerciseKcal, nominalExerciseKcal - netExerciseKcal)
                        } else null,
                        accentColor = AppleColors.MintGreen,
                    )
                }

                // 三环 Canvas 绘制
                val intakeRatio = if (baseTdeeKcal > 0) (totalIntakeKcal / baseTdeeKcal).toFloat() else 0f
                val exerciseRatio = (netExerciseKcal / 400.0).toFloat()
                val waterRatio = (waterProgressPct / 100.0).toFloat()

                AppleActivityRings(
                    intakeProgress = intakeRatio,
                    exerciseProgress = exerciseRatio,
                    waterProgress = waterRatio,
                    size = 136.dp,
                    strokeWidth = 11.dp,
                )
            }

            Spacer(modifier = Modifier.height(4.dp))

            // 净能量差底栏
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(14.dp))
                    .background(
                        if (netEnergyDiff <= 0) AppleColors.MintGreenSoft else AppleColors.VitalityCoralSoft,
                    )
                    .padding(horizontal = 14.dp, vertical = 10.dp),
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Row(
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            text = if (netEnergyDiff <= 0) "当前处于热量缺口" else "当前处于热量盈余",
                            style = MaterialTheme.typography.labelLarge.copy(
                                fontWeight = FontWeight.SemiBold,
                                color = if (netEnergyDiff <= 0) AppleColors.MintGreen else AppleColors.VitalityCoral,
                            ),
                        )
                        if (AppStateManager.dayBoundaryMinutes > 0u) {
                            CapsuleBadge(
                                text = "日界线 04:00",
                                color = AppleColors.AmberGold,
                                backgroundColor = AppleColors.AmberGoldSoft,
                            )
                        }
                    }

                    Text(
                        text = "${if (netEnergyDiff > 0) "+" else ""}${"%.0f".format(netEnergyDiff)} kcal",
                        style = MaterialTheme.typography.titleMedium.copy(
                            fontWeight = FontWeight.Bold,
                            color = if (netEnergyDiff <= 0) AppleColors.MintGreen else AppleColors.VitalityCoral,
                        ),
                    )
                }
            }
        }

        // 3. 每日提示（演示规则文案，非真实模型）
        val aiInsight = AiHealthAssistant.generateDailyReflection(
            netEnergyDifferenceKcal = netEnergyDiff,
            waterPct = waterProgressPct.toFloat(),
            boundaryShiftActive = AppStateManager.dayBoundaryMinutes > 0u,
        )

        currentUser.energyPlanningBlockReason()?.let { reason ->
            AppleCard {
                CapsuleBadge(text = "能量规划已禁用", color = AppleColors.VitalityCoral)
                Text(
                    text = reason,
                    style = MaterialTheme.typography.bodyMedium.copy(fontWeight = FontWeight.SemiBold),
                )
            }
        }

        AiInsightCard(
            title = "今日提示（演示规则）",
            insight = aiInsight,
            badgeText = "演示规则",
            actionLabel = if (AppStateManager.dayBoundaryMinutes == 0u) "切换至跨午夜日界线" else "恢复自然日界线",
            onAction = {
                AppStateManager.dayBoundaryMinutes = if (AppStateManager.dayBoundaryMinutes == 0u) 240u else 0u
            },
        )

        // 4. 三大宏量营养素分布卡片 (Macronutrients Tracker)
        AppleCard(customGlowBrush = LiquidGlassTokens.MintLiquidGlow) {
            Text(
                text = "宏量营养素摄入",
                style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold),
            )

            Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                // 蛋白质
                MacroProgressRow(
                    label = "蛋白质 (Protein)",
                    currentG = proteinG,
                    targetG = 130.0,
                    color = AppleColors.MintGreen,
                )
                // 碳水
                MacroProgressRow(
                    label = "慢碳水 (Carbs)",
                    currentG = carbsG,
                    targetG = 180.0,
                    color = AppleColors.AmberGold,
                )
                // 脂肪
                MacroProgressRow(
                    label = "优质脂肪 (Fat)",
                    currentG = fatG,
                    targetG = 55.0,
                    color = AppleColors.IrisPurple,
                )
            }
        }

        // 5. 饮水与补水微件卡片
        AppleCard(customGlowBrush = LiquidGlassTokens.IndigoLiquidGlow) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Row(
                    horizontalArrangement = Arrangement.spacedBy(10.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Box(
                        modifier = Modifier
                            .size(40.dp)
                            .clip(CircleShape)
                            .background(AppleColors.CalmIndigoSoft),
                        contentAlignment = Alignment.Center,
                    ) {
                        Icon(
                            imageVector = Icons.Rounded.LocalDrink,
                            contentDescription = "饮水",
                            tint = AppleColors.CalmIndigo,
                            modifier = Modifier.size(22.dp),
                        )
                    }

                    Column {
                        Text(
                            text = "生理补水进度",
                            style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.SemiBold),
                        )
                        Text(
                            text = "$waterCurrentMl / $waterGoalMl ml · ${"%.0f".format(waterProgressPct)}%",
                            style = MaterialTheme.typography.labelMedium.copy(
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            ),
                        )
                    }
                }

                // 快速记水 +250ml 药丸按钮
                Box(
                    modifier = Modifier
                        .clip(CircleShape)
                        .background(AppleColors.CalmIndigoSoft)
                        .clickable {
                            scope.launch {
                                runCatching {
                                    withContext(Dispatchers.IO) {
                                        val session = CoreSession.get(context)
                                        val stamp = LocalDateTime.now().format(DateTimeFormatter.ofPattern("yyyy-MM-dd'T'HH:mm:ss"))
                                        session.logWater(currentUser.userId, 250u, stamp)
                                    }
                                }.onSuccess {
                                    loadTodayData()
                                }
                            }
                        }
                        .padding(horizontal = 12.dp, vertical = 7.dp),
                    contentAlignment = Alignment.Center,
                ) {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        Icon(
                            imageVector = Icons.Rounded.Add,
                            contentDescription = "记水",
                            tint = AppleColors.CalmIndigo,
                            modifier = Modifier.size(16.dp),
                        )
                        Text(
                            text = "+250ml",
                            style = MaterialTheme.typography.labelMedium.copy(
                                color = AppleColors.CalmIndigo,
                                fontWeight = FontWeight.Bold,
                            ),
                        )
                    }
                }
            }

            LinearProgressIndicator(
                progress = { (waterProgressPct / 100.0).toFloat().coerceIn(0f, 1f) },
                modifier = Modifier
                    .fillMaxWidth()
                    .height(8.dp)
                    .clip(CircleShape),
                color = AppleColors.CalmIndigo,
                trackColor = AppleColors.CalmIndigoSoft,
            )
        }

        // 6. 底部状态与日界线提示
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(vertical = 8.dp),
            horizontalArrangement = Arrangement.Center,
        ) {
            Text(
                text = statusMessage,
                style = MaterialTheme.typography.labelSmall.copy(
                    color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f),
                ),
            )
        }
    }
}

@Composable
private fun MacroProgressRow(
    label: String,
    currentG: Double,
    targetG: Double,
    color: Color,
) {
    val ratio = if (targetG > 0) (currentG / targetG).toFloat().coerceIn(0f, 1.5f) else 0f

    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            Text(
                text = label,
                style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.Medium),
            )
            Text(
                text = "${"%.0f".format(currentG)} / ${"%.0f".format(targetG)}g",
                style = MaterialTheme.typography.labelMedium.copy(
                    fontWeight = FontWeight.SemiBold,
                    color = MaterialTheme.colorScheme.onSurface,
                ),
            )
        }
        LinearProgressIndicator(
            progress = { ratio.coerceAtMost(1f) },
            modifier = Modifier
                .fillMaxWidth()
                .height(6.5.dp)
                .clip(CircleShape),
            color = color,
            trackColor = color.copy(alpha = 0.15f),
        )
    }
}
