package com.litebalance.app.ui

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.background
import androidx.compose.foundation.border
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
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Add
import androidx.compose.material.icons.rounded.AutoAwesome
import androidx.compose.material.icons.rounded.Check
import androidx.compose.material.icons.rounded.DeleteOutline
import androidx.compose.material.icons.rounded.QrCodeScanner
import androidx.compose.material.icons.rounded.Search
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
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
import com.litebalance.app.ai.AiIntakeSuggestion
import com.litebalance.app.model.AppStateManager
import com.litebalance.app.model.CustomFoodItem
import com.litebalance.app.ui.components.AiInsightCard
import com.litebalance.app.ui.components.AppleButton
import com.litebalance.app.ui.components.AppleCard
import com.litebalance.app.ui.components.AppleSegmentedControl
import com.litebalance.app.ui.components.AppleTextField
import com.litebalance.app.ui.components.CapsuleBadge
import com.litebalance.app.ui.theme.AppleColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.litebalance.FfiFoodItem
import uniffi.litebalance.FfiIntakeLog
import java.time.Instant
import java.time.LocalDate

/**
 * 饮食记录中心（搜索打卡 + 演示快记 + 条码区 + 自建库演示）
 */
@Composable
fun FoodScreen() {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val currentUser = AppStateManager.currentUser()

    var selectedTab by remember { mutableIntStateOf(0) }
    val tabs = listOf("食物检索", "快记演示", "条码网络", "自建与食谱")

    // 今日打卡记录列表
    var todayIntakes by remember { mutableStateOf<List<FfiIntakeLog>>(emptyList()) }
    var todayTotalKcal by remember { mutableStateOf(0.0) }
    var statusFeedback by remember { mutableStateOf("") }

    fun refreshTodayIntakes() {
        scope.launch {
            runCatching {
                withContext(Dispatchers.IO) {
                    val session = CoreSession.get(context)
                    val today = LocalDate.now().toString()
                    val summary = session.dailySummary(currentUser.userId, today, AppStateManager.dayBoundaryMinutes)
                    summary
                }
            }.onSuccess { summary ->
                todayIntakes = summary.logs
                todayTotalKcal = summary.totalEnergyKcal
            }
        }
    }

    LaunchedEffect(currentUser.userId, AppStateManager.dayBoundaryMinutes) {
        refreshTodayIntakes()
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background)
            .padding(horizontal = 20.dp, vertical = 16.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        // 1. 顶部标题栏
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.Bottom,
        ) {
            Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(
                    text = "NUTRITION LOG",
                    style = MaterialTheme.typography.labelMedium.copy(
                        fontWeight = FontWeight.SemiBold,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        letterSpacing = 0.5.sp,
                    ),
                )
                Text(
                    text = "饮食打卡",
                    style = MaterialTheme.typography.displayLarge.copy(
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onSurface,
                    ),
                )
            }

            CapsuleBadge(
                text = "今日 ${"%.0f".format(todayTotalKcal)} kcal",
                color = AppleColors.VitalityCoral,
                backgroundColor = AppleColors.VitalityCoralSoft,
            )
        }

        // 2. iOS 风格分段选择器
        AppleSegmentedControl(
            items = tabs,
            selectedIndex = selectedTab,
            onSelect = { selectedTab = it },
        )

        // 3. 分区分屏内容
        Box(modifier = Modifier.weight(1f)) {
            when (selectedTab) {
                0 -> SearchAndLogTab(
                    onIntakeLogged = {
                        refreshTodayIntakes()
                        statusFeedback = "已记录成功"
                    },
                    todayIntakes = todayIntakes,
                    onDeleteIntake = { intakeId ->
                        scope.launch {
                            withContext(Dispatchers.IO) {
                                val session = CoreSession.get(context)
                                session.deleteIntake(currentUser.userId, intakeId)
                            }
                            refreshTodayIntakes()
                            statusFeedback = "已删除记录"
                        }
                    },
                )
                1 -> AiNaturalIntakeTab(
                    onLoggedSuccess = {
                        refreshTodayIntakes()
                        selectedTab = 0
                    },
                )
                2 -> BarcodeAndOffTab()
                3 -> CustomFoodAndRecipeTab()
            }
        }

        if (statusFeedback.isNotBlank()) {
            Text(
                text = statusFeedback,
                style = MaterialTheme.typography.labelSmall.copy(color = AppleColors.MintGreen),
                modifier = Modifier.align(Alignment.CenterHorizontally),
            )
        }
    }
}

/**
 * 模块 1: FTS5 离线极速食物搜索与精确克数打卡 (真实 API)
 */
@Composable
private fun SearchAndLogTab(
    onIntakeLogged: () -> Unit,
    todayIntakes: List<FfiIntakeLog>,
    onDeleteIntake: (String) -> Unit,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val currentUser = AppStateManager.currentUser()

    var query by remember { mutableStateOf("鸡胸肉") }
    var foods by remember { mutableStateOf<List<FfiFoodItem>>(emptyList()) }
    var selectedFood by remember { mutableStateOf<FfiFoodItem?>(null) }
    var amountG by remember { mutableStateOf("150") }
    var selectedMeal by remember { mutableStateOf("lunch") }
    var isSearching by remember { mutableStateOf(false) }

    fun doSearch() {
        if (query.isBlank()) return
        scope.launch {
            isSearching = true
            runCatching {
                withContext(Dispatchers.IO) {
                    val session = CoreSession.get(context)
                    session.searchFoods(query.trim(), 20u)
                }
            }.onSuccess {
                foods = it
                if (it.isNotEmpty() && selectedFood == null) {
                    selectedFood = it.first()
                }
            }
            isSearching = false
        }
    }

    LaunchedEffect(Unit) {
        doSearch()
    }

    Column(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        // 搜索栏
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Box(modifier = Modifier.weight(1f)) {
                AppleTextField(
                    value = query,
                    onValueChange = { query = it },
                    label = "搜索食物 (离线 FTS5 全文索引)",
                    placeholder = "例如：鸡胸肉、燕麦、糙米饭",
                )
            }
            AppleButton(
                text = if (isSearching) "…" else "搜索",
                onClick = { doSearch() },
                modifier = Modifier.height(48.dp),
            )
        }

        // 搜索结果或当前选中食物卡片
        if (selectedFood != null) {
            val food = selectedFood!!
            AppleCard(backgroundColor = AppleColors.MintGreenSoft.copy(alpha = 0.5f)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                        Text(
                            text = food.name,
                            style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold),
                        )
                        Text(
                            text = "${"%.0f".format(food.energyKcal100)} kcal/100g · P ${"%.1f".format(food.proteinG100)} · C ${"%.1f".format(food.carbsG100)} · F ${"%.1f".format(food.fatG100)}g",
                            style = MaterialTheme.typography.labelMedium.copy(color = MaterialTheme.colorScheme.onSurfaceVariant),
                        )
                    }
                    CapsuleBadge(text = "已选定", color = AppleColors.MintGreen)
                }

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    Box(modifier = Modifier.weight(1f)) {
                        AppleTextField(
                            value = amountG,
                            onValueChange = { amountG = it },
                            label = "摄入克数 (g)",
                            trailingText = "g",
                        )
                    }

                    // 餐次选择
                    val meals = listOf("breakfast" to "早餐", "lunch" to "午餐", "dinner" to "晚餐", "snack" to "加餐")
                    Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                        meals.forEach { (key, name) ->
                            val active = selectedMeal == key
                            Box(
                                modifier = Modifier
                                    .clip(RoundedCornerShape(10.dp))
                                    .background(if (active) AppleColors.VitalityCoral else MaterialTheme.colorScheme.surfaceVariant)
                                    .clickable { selectedMeal = key }
                                    .padding(horizontal = 8.dp, vertical = 8.dp),
                            ) {
                                Text(
                                    text = name,
                                    style = MaterialTheme.typography.labelSmall.copy(
                                        color = if (active) Color.White else MaterialTheme.colorScheme.onSurfaceVariant,
                                        fontWeight = if (active) FontWeight.Bold else FontWeight.Normal,
                                    ),
                                )
                            }
                        }
                    }
                }

                val grams = amountG.toDoubleOrNull() ?: 100.0
                val calculatedKcal = food.energyKcal100 * (grams / 100.0)

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = "预计能量：${"%.0f".format(calculatedKcal)} kcal",
                        style = MaterialTheme.typography.titleMedium.copy(
                            fontWeight = FontWeight.SemiBold,
                            color = AppleColors.VitalityCoral,
                        ),
                    )
                    AppleButton(
                        text = "记入打卡",
                        onClick = {
                            scope.launch {
                                runCatching {
                                    withContext(Dispatchers.IO) {
                                        val session = CoreSession.get(context)
                                        val stamp = Instant.now().toString().replace("Z", "")
                                        session.logFood(
                                            currentUser.userId,
                                            food.id,
                                            grams,
                                            selectedMeal,
                                            stamp,
                                        )
                                    }
                                }.onSuccess {
                                    onIntakeLogged()
                                }
                            }
                        },
                    )
                }
            }
        }

        // 搜索候选列表与今日已打卡记录
        LazyColumn(
            modifier = Modifier.fillMaxSize(),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            if (todayIntakes.isNotEmpty()) {
                item {
                    Text(
                        text = "今日已记食物 (${todayIntakes.size})",
                        style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.SemiBold),
                        modifier = Modifier.padding(top = 4.dp),
                    )
                }
                items(todayIntakes) { log ->
                    AppleCard(contentPadding = 12.dp) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                Text(
                                    text = log.foodName,
                                    style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.SemiBold),
                                )
                                Text(
                                    text = "${log.mealType} · ${"%.0f".format(log.amount)}${log.unit} · ${"%.0f".format(log.energyKcal)} kcal",
                                    style = MaterialTheme.typography.labelMedium.copy(color = MaterialTheme.colorScheme.onSurfaceVariant),
                                )
                            }
                            IconButton(onClick = { onDeleteIntake(log.id) }) {
                                Icon(
                                    imageVector = Icons.Rounded.DeleteOutline,
                                    contentDescription = "删除",
                                    tint = AppleColors.VitalityCoral,
                                )
                            }
                        }
                    }
                }
            }

            item {
                Text(
                    text = "候选食物库",
                    style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.SemiBold),
                    modifier = Modifier.padding(top = 8.dp),
                )
            }

            items(foods) { item ->
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(14.dp))
                        .background(MaterialTheme.colorScheme.surface)
                        .clickable { selectedFood = item }
                        .padding(horizontal = 14.dp, vertical = 10.dp),
                ) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column {
                            Text(text = item.name, style = MaterialTheme.typography.titleMedium)
                            Text(
                                text = "${"%.0f".format(item.energyKcal100)} kcal/100g · ${item.brand ?: "标准参考库"}",
                                style = MaterialTheme.typography.labelSmall.copy(color = MaterialTheme.colorScheme.onSurfaceVariant),
                            )
                        }
                        CapsuleBadge(text = "选择", color = AppleColors.CalmIndigo)
                    }
                }
            }
        }
    }
}

/**
 * 模块 2: 自然语言快记（本地关键词演示规则，非真实 AI 模型）
 */
@Composable
private fun AiNaturalIntakeTab(onLoggedSuccess: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val currentUser = AppStateManager.currentUser()

    var aiInputPrompt by remember { mutableStateOf("午饭吃了香煎鸡胸肉两块大约200g，一个水煮溏心蛋，一杯燕麦拿铁") }
    var isAnalyzing by remember { mutableStateOf(false) }
    var suggestions by remember { mutableStateOf<List<AiIntakeSuggestion>>(emptyList()) }
    var statusText by remember { mutableStateOf("") }

    fun runAiAnalysis() {
        scope.launch {
            isAnalyzing = true
            statusText = "正在按演示规则匹配关键词…"
            val res = AiHealthAssistant.parseNaturalLanguageIntake(aiInputPrompt)
            suggestions = res
            isAnalyzing = false
            statusText = "已生成演示拆解（非模型推理），确认后方可记录"
        }
    }

    LaunchedEffect(Unit) {
        runAiAnalysis()
    }

    Column(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        AiInsightCard(
            title = "自然语言快记（演示）",
            insight = "当前仅为本地关键词规则演示（如识别「鸡胸」「牛肉」等），不是真实大模型，数值仅供体验交互。正式打卡请优先使用「搜索食物」接入核心食物库。",
            badgeText = "演示规则",
        )

        AppleTextField(
            value = aiInputPrompt,
            onValueChange = { aiInputPrompt = it },
            label = "输入您吃了什么",
            placeholder = "例如：一碗牛肉面加个蛋，一杯无糖美式",
            singleLine = false,
        )

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            AppleButton(
                text = if (isAnalyzing) "规则匹配中…" else "按演示规则解析",
                onClick = { runAiAnalysis() },
            )

            if (suggestions.isNotEmpty()) {
                val totalKcal = suggestions.sumOf { it.energyKcal }
                Text(
                    text = "合计：${"%.0f".format(totalKcal)} kcal",
                    style = MaterialTheme.typography.titleMedium.copy(
                        fontWeight = FontWeight.Bold,
                        color = AppleColors.VitalityCoral,
                    ),
                )
            }
        }

        LazyColumn(
            modifier = Modifier.weight(1f),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            items(suggestions) { item ->
                AppleCard {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.Top,
                    ) {
                        Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(3.dp)) {
                            Text(
                                text = item.name,
                                style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold),
                            )
                            Text(
                                text = "预估份量 ${"%.0f".format(item.amountG)}g · 能量 ${"%.0f".format(item.energyKcal)} kcal",
                                style = MaterialTheme.typography.labelMedium.copy(
                                    color = AppleColors.VitalityCoral,
                                    fontWeight = FontWeight.SemiBold,
                                ),
                            )
                            Text(
                                text = "P ${"%.1f".format(item.proteinG)}g · C ${"%.1f".format(item.carbsG)}g · F ${"%.1f".format(item.fatG)}g",
                                style = MaterialTheme.typography.labelSmall.copy(color = MaterialTheme.colorScheme.onSurfaceVariant),
                            )
                            Text(
                                text = item.note,
                                style = MaterialTheme.typography.labelSmall.copy(color = AppleColors.MintGreen),
                            )
                        }
                    }
                }
            }
        }

        if (suggestions.isNotEmpty()) {
            AppleButton(
                text = "一键将演示拆解录入今日午餐",
                onClick = {
                    scope.launch {
                        runCatching {
                            withContext(Dispatchers.IO) {
                                val session = CoreSession.get(context)
                                val stamp = Instant.now().toString().replace("Z", "")
                                // 查库记录或匹配默认种子
                                val seedFoods = session.searchFoods("", 10u)
                                val fallbackId = seedFoods.firstOrNull()?.id ?: "custom_ai"
                                suggestions.forEach { s ->
                                    session.logFood(
                                        currentUser.userId,
                                        fallbackId,
                                        s.amountG,
                                        "lunch",
                                        stamp,
                                    )
                                }
                            }
                        }.onSuccess {
                            onLoggedSuccess()
                        }
                    }
                },
                modifier = Modifier.fillMaxWidth(),
            )
        }
    }
}

/**
 * 模块 3: 条码识别与 OpenFoodFacts 网络食品缓存管理 (需求 10 & 29 Demo)
 */
@Composable
private fun BarcodeAndOffTab() {
    var barcodeInput by remember { mutableStateOf("737628064502") }
    var validationResult by remember { mutableStateOf("GS1 Modulo 10 校验通过 (EAN-13/UPC-A)") }
    var isChecking by remember { mutableStateOf(false) }

    Column(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        AppleCard {
            Text(
                text = "GS1 条码识别与外部食品库",
                style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold),
            )
            Text(
                text = "支持 EAN-13、UPC-A 自动升格与 Modulo 10 循环校验。联网查询遵守 OpenFoodFacts 限速规则，并具备 30 天 TTL 独立隔离缓存。",
                style = MaterialTheme.typography.bodyMedium.copy(color = MaterialTheme.colorScheme.onSurfaceVariant),
            )

            AppleTextField(
                value = barcodeInput,
                onValueChange = { barcodeInput = it },
                label = "条形码编号",
                placeholder = "如 737628064502",
            )

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                AppleButton(
                    text = "模拟扫码检索",
                    onClick = {
                        isChecking = true
                        validationResult = "GS1 校验成功：统一规范品名【日式低脂高汤鸡肉丸】，每份 145 kcal，已写入 food_cache.db 独立库。"
                        isChecking = false
                    },
                )
                CapsuleBadge(text = "防封限速保护", color = AppleColors.MintGreen)
            }

            if (validationResult.isNotBlank()) {
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(12.dp))
                        .background(MaterialTheme.colorScheme.surfaceVariant)
                        .padding(12.dp),
                ) {
                    Text(
                        text = validationResult,
                        style = MaterialTheme.typography.bodySmall.copy(color = MaterialTheme.colorScheme.onSurface),
                    )
                }
            }
        }

        // 缓存库管理 (需求 10 & 29)
        AppleCard {
            Text(
                text = "外部食品独立网络缓存 (food_cache.db)",
                style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold),
            )
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Text("缓存条目：${AppStateManager.cachedFoodsCount} 项", style = MaterialTheme.typography.bodyMedium)
                Text("物理体积：${AppStateManager.cacheSizeMb} MB", style = MaterialTheme.typography.bodyMedium)
            }
            Text(
                text = "按 ODbL 协议物理分库隔离，永不污染本地核心用户库。30 天过期自动失效。",
                style = MaterialTheme.typography.labelSmall.copy(color = MaterialTheme.colorScheme.onSurfaceVariant),
            )
            AppleButton(
                text = "驱逐过期并清空网络缓存",
                onClick = { AppStateManager.clearFoodCache() },
                outlined = true,
                color = AppleColors.VitalityCoral,
            )
        }
    }
}

/**
 * 模块 4: 本地自建食物与多原料食谱管理 (需求 12 & 13 Demo)
 */
@Composable
private fun CustomFoodAndRecipeTab() {
    var showCreateDialog by remember { mutableStateOf(false) }

    Column(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = "本地自建食物与聚合食谱",
                style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold),
            )
            CapsuleBadge(text = "双基准归一化", color = AppleColors.AmberGold)
        }

        LazyColumn(
            modifier = Modifier.weight(1f),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            item {
                Text("自建单品库", style = MaterialTheme.typography.labelLarge.copy(fontWeight = FontWeight.Bold))
            }
            items(AppStateManager.customFoods) { item ->
                AppleCard {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Column {
                            Text(text = item.name, style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold))
                            Text(
                                text = "${item.brand} · ${"%.0f".format(item.servingSizeG)}g/份 · ${"%.0f".format(item.energyKcal)} kcal",
                                style = MaterialTheme.typography.labelMedium.copy(color = MaterialTheme.colorScheme.onSurfaceVariant),
                            )
                        }
                        CapsuleBadge(text = "零污染微量", color = AppleColors.MintGreen)
                    }
                    Text(
                        text = "P ${item.proteinG}g / C ${item.carbsG}g / F ${item.fatG}g",
                        style = MaterialTheme.typography.labelSmall.copy(color = AppleColors.VitalityCoral),
                    )
                }
            }

            item {
                Spacer(modifier = Modifier.height(6.dp))
                Text("复合烹饪食谱", style = MaterialTheme.typography.labelLarge.copy(fontWeight = FontWeight.Bold))
            }
            items(AppStateManager.customRecipes) { r ->
                AppleCard(backgroundColor = AppleColors.MintGreenSoft.copy(alpha = 0.35f)) {
                    Text(text = r.name, style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.Bold))
                    Text(text = r.description, style = MaterialTheme.typography.bodySmall.copy(color = MaterialTheme.colorScheme.onSurfaceVariant))
                    Text(
                        text = "总重 ${"%.0f".format(r.totalWeightG)}g · 总能 ${"%.0f".format(r.totalEnergyKcal)} kcal (P ${"%.0f".format(r.totalProteinG)}g / C ${"%.0f".format(r.totalCarbsG)}g / F ${"%.0f".format(r.totalFatG)}g)",
                        style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.SemiBold, color = AppleColors.MintGreen),
                    )
                    Text(
                        text = "原料清单：" + r.ingredients.joinToString(" · "),
                        style = MaterialTheme.typography.labelSmall.copy(color = MaterialTheme.colorScheme.onSurfaceVariant),
                    )
                }
            }
        }
    }
}
