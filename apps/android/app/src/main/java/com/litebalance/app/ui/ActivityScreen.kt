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
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Bedtime
import androidx.compose.material.icons.rounded.FitnessCenter
import androidx.compose.material.icons.rounded.MonitorWeight
import androidx.compose.material.icons.rounded.PlayArrow
import androidx.compose.material.icons.rounded.Stop
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
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
import com.litebalance.app.model.ActivityItem
import com.litebalance.app.model.AppStateManager
import com.litebalance.app.model.FastingProtocol
import com.litebalance.app.ui.components.AppleButton
import com.litebalance.app.ui.components.AppleCard
import com.litebalance.app.ui.components.AppleMetricItem
import com.litebalance.app.ui.components.AppleSegmentedControl
import com.litebalance.app.ui.components.AppleTextField
import com.litebalance.app.ui.components.CapsuleBadge
import com.litebalance.app.ui.theme.AppleColors
import com.litebalance.app.ui.theme.LiquidGlassTokens
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.litebalance.FfiWeightPoint
import java.time.LocalDateTime
import java.time.format.DateTimeFormatter

/**
 * 活力与节律中心 (间歇性断食呼吸环、Herrmann 运动知识库与 Pontzer 代偿、体重与体脂趋势)
 */
@Composable
fun ActivityScreen() {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val currentUser = AppStateManager.currentUser()

    var selectedTab by remember { mutableIntStateOf(0) }
    val tabs = listOf("间歇性断食", "运动与能量代偿", "体重与体脂")

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background)
            .padding(horizontal = 20.dp, vertical = 16.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        // 顶部标题
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.Bottom,
        ) {
            Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(
                    text = "VITALITY & RHYTHM",
                    style = MaterialTheme.typography.labelMedium.copy(
                        fontWeight = FontWeight.SemiBold,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        letterSpacing = 0.5.sp,
                    ),
                )
                Text(
                    text = "活力与节律",
                    style = MaterialTheme.typography.displayLarge.copy(
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onSurface,
                    ),
                )
            }
            CapsuleBadge(text = "Pontzer 2026", color = AppleColors.MintGreen)
        }

        AppleSegmentedControl(
            items = tabs,
            selectedIndex = selectedTab,
            onSelect = { selectedTab = it },
        )

        Box(modifier = Modifier.weight(1f)) {
            when (selectedTab) {
                0 -> FastingTab()
                1 -> WorkoutAndCompensationTab()
                2 -> WeightHistoryTab()
            }
        }
    }
}

/**
 * 分区 1: 间歇性断食呼吸环计时器 (需求 16 Demo)
 */
@Composable
private fun FastingTab() {
    val session = AppStateManager.fastingSession
    val scrollState = rememberScrollState()

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(scrollState),
        verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        AppleCard(customGlowBrush = LiquidGlassTokens.AmberLiquidGlow) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.Top,
            ) {
                Column {
                    Text(
                        text = session.protocol.label,
                        style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold),
                    )
                    Text(
                        text = "断食 ${session.protocol.fastHours} 小时 · 进食窗口 ${session.protocol.windowHours} 小时",
                        style = MaterialTheme.typography.labelMedium.copy(color = MaterialTheme.colorScheme.onSurfaceVariant),
                    )
                }
                CapsuleBadge(
                    text = if (session.isRunning) "断食进行中" else "未开始",
                    color = if (session.isRunning) AppleColors.AmberGold else AppleColors.SlatePebble,
                )
            }

            // 苹果风格环形断食呼吸进度
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(vertical = 14.dp),
                contentAlignment = Alignment.Center,
            ) {
                Box(contentAlignment = Alignment.Center) {
                    CircularProgressIndicator(
                        progress = { session.progressPct() },
                        modifier = Modifier.size(190.dp),
                        strokeWidth = 14.dp,
                        color = AppleColors.AmberGold,
                        trackColor = AppleColors.AmberGoldSoft,
                    )
                    Column(
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        val hours = session.elapsedMinutes() / 60
                        val mins = session.elapsedMinutes() % 60
                        Text(
                            text = "%02d:%02d".format(hours, mins),
                            style = MaterialTheme.typography.displayLarge.copy(
                                fontWeight = FontWeight.Bold,
                                color = AppleColors.AmberGold,
                            ),
                        )
                        Text(
                            text = session.currentPhase().label,
                            style = MaterialTheme.typography.labelLarge.copy(
                                fontWeight = FontWeight.SemiBold,
                                color = MaterialTheme.colorScheme.onSurface,
                            ),
                        )
                    }
                }
            }

            Text(
                text = "生理阶段：" + session.currentPhase().description,
                style = MaterialTheme.typography.bodyMedium.copy(color = MaterialTheme.colorScheme.onSurfaceVariant),
            )

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                if (!session.isRunning) {
                    AppleButton(
                        text = "开启断食计时",
                        onClick = { AppStateManager.startFasting(FastingProtocol.F16_8) },
                        modifier = Modifier.fillMaxWidth(),
                        color = AppleColors.AmberGold,
                    )
                } else {
                    AppleButton(
                        text = "完成 / 结束断食",
                        onClick = { AppStateManager.stopFasting() },
                        modifier = Modifier.weight(1f),
                        color = AppleColors.MintGreen,
                    )
                    AppleButton(
                        text = "取消",
                        onClick = { AppStateManager.stopFasting() },
                        modifier = Modifier.weight(1f),
                        outlined = true,
                        color = AppleColors.VitalityCoral,
                    )
                }
            }
        }

        // 协议选择卡片
        AppleCard {
            Text("断食协议选择", style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold))
            FastingProtocol.entries.forEach { proto ->
                val isSel = session.protocol == proto
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(12.dp))
                        .background(if (isSel) AppleColors.AmberGoldSoft else MaterialTheme.colorScheme.surfaceVariant)
                        .clickable { AppStateManager.startFasting(proto) }
                        .padding(12.dp),
                ) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(text = proto.label, style = MaterialTheme.typography.bodyMedium.copy(fontWeight = FontWeight.SemiBold))
                        if (isSel) CapsuleBadge(text = "使用中", color = AppleColors.AmberGold)
                    }
                }
            }
        }
    }
}

/**
 * 分区 2: Herrmann 运动库与 Pontzer 能量代偿模型 (需求 19, 20, 21 Demo)
 */
@Composable
private fun WorkoutAndCompensationTab() {
    val currentUser = AppStateManager.currentUser()
    var selectedAct by remember { mutableStateOf<ActivityItem?>(AppStateManager.standardActivities.first()) }
    var durationMinutes by remember { mutableStateOf("45") }

    Column(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        AppleCard {
            Text(
                text = "Pontzer-Trexler 2026 运动代偿感知天平",
                style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold),
            )
            Text(
                text = "运动不是简单的线性加法！人体具有能量代偿机制（挤压非活动性生热与免疫代谢）。本模块将毛消耗扣除代偿阻尼，计算真实净计入，避免高估能耗引发反弹暴食。",
                style = MaterialTheme.typography.bodySmall.copy(color = MaterialTheme.colorScheme.onSurfaceVariant),
            )

            val act = selectedAct ?: AppStateManager.standardActivities.first()
            val dur = durationMinutes.toIntOrNull() ?: 30
            val nominal = act.met * 1.05 * currentUser.weightKg * (dur / 60.0)
            val compPct = if (act.category == "力量") 20 else 28
            val net = nominal * (1.0 - compPct / 100.0)

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                AppleMetricItem(
                    label = "名义毛消耗 (手表显示)",
                    value = "%.0f".format(nominal),
                    unit = "kcal",
                    accentColor = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                AppleMetricItem(
                    label = "Pontzer 真实净计入",
                    value = "%.0f".format(net),
                    unit = "kcal",
                    accentColor = AppleColors.MintGreen,
                )
                AppleMetricItem(
                    label = "代偿挤压阻尼",
                    value = "-$compPct",
                    unit = "%",
                    accentColor = AppleColors.VitalityCoral,
                )
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(10.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Box(modifier = Modifier.weight(1f)) {
                    AppleTextField(
                        value = durationMinutes,
                        onValueChange = { durationMinutes = it },
                        label = "运动时长 (分钟)",
                        trailingText = "min",
                    )
                }
                AppleButton(
                    text = "打卡记入闭环",
                    onClick = {
                        AppStateManager.logWorkout(act, dur, currentUser.weightKg)
                    },
                )
            }
        }

        Text(
            text = "Herrmann 2024 临床运动库",
            style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold),
        )

        LazyColumn(
            modifier = Modifier.weight(1f),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            items(AppStateManager.standardActivities) { item ->
                val isSel = selectedAct?.id == item.id
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(14.dp))
                        .background(if (isSel) AppleColors.MintGreenSoft else MaterialTheme.colorScheme.surface)
                        .clickable { selectedAct = item }
                        .padding(horizontal = 14.dp, vertical = 12.dp),
                ) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column {
                            Text(text = item.name, style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.SemiBold))
                            Text(
                                text = "${item.category} · MET ${item.met} · 默认 ${item.defaultDurationMin} 分钟",
                                style = MaterialTheme.typography.labelSmall.copy(color = MaterialTheme.colorScheme.onSurfaceVariant),
                            )
                        }
                        CapsuleBadge(text = "选择", color = AppleColors.MintGreen)
                    }
                }
            }
        }
    }
}

/**
 * 分区 3: 体重与体脂率打卡与历史记录 (真实 API)
 */
@Composable
private fun WeightHistoryTab() {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val currentUser = AppStateManager.currentUser()

    var inputWeight by remember { mutableStateOf("64.5") }
    var inputBodyFat by remember { mutableStateOf("22.0") }
    var historyPoints by remember { mutableStateOf<List<FfiWeightPoint>>(emptyList()) }
    var statusText by remember { mutableStateOf("") }

    fun refreshHistory() {
        scope.launch {
            runCatching {
                withContext(Dispatchers.IO) {
                    val session = CoreSession.get(context)
                    session.weightHistory(currentUser.userId, 30u)
                }
            }.onSuccess {
                historyPoints = it
            }
        }
    }

    LaunchedEffect(currentUser.userId) {
        refreshHistory()
    }

    Column(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        AppleCard {
            Text(
                text = "记录今日体重与体脂",
                style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold),
            )

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                Box(modifier = Modifier.weight(1f)) {
                    AppleTextField(value = inputWeight, onValueChange = { inputWeight = it }, label = "体重 (kg)", trailingText = "kg")
                }
                Box(modifier = Modifier.weight(1f)) {
                    AppleTextField(value = inputBodyFat, onValueChange = { inputBodyFat = it }, label = "体脂率 (%)", trailingText = "%")
                }
            }

            AppleButton(
                text = "打卡保存体重记录",
                onClick = {
                    scope.launch {
                        runCatching {
                            withContext(Dispatchers.IO) {
                                val session = CoreSession.get(context)
                                val stamp = LocalDateTime.now().format(DateTimeFormatter.ofPattern("yyyy-MM-dd'T'HH:mm:ss"))
                                session.logWeight(
                                    currentUser.userId,
                                    inputWeight.toDoubleOrNull() ?: 65.0,
                                    inputBodyFat.toDoubleOrNull(),
                                    stamp,
                                )
                            }
                        }.onSuccess {
                            refreshHistory()
                            statusText = "已同步至 SQLite 核心数据库"
                        }
                    }
                },
                modifier = Modifier.fillMaxWidth(),
            )

            if (statusText.isNotBlank()) {
                Text(text = statusText, style = MaterialTheme.typography.labelSmall.copy(color = AppleColors.MintGreen))
            }
        }

        Text(
            text = "近 30 日体重趋势记录 (${historyPoints.size})",
            style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold),
        )

        LazyColumn(
            modifier = Modifier.weight(1f),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            items(historyPoints) { pt ->
                AppleCard(contentPadding = 12.dp) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column {
                            Text(text = pt.date, style = MaterialTheme.typography.bodyMedium.copy(fontWeight = FontWeight.SemiBold))
                            Text(
                                text = "体重 ${"%.2f".format(pt.weightKg)} kg",
                                style = MaterialTheme.typography.labelMedium.copy(color = AppleColors.VitalityCoral, fontWeight = FontWeight.Bold),
                            )
                        }
                    }
                }
            }
        }
    }
}
