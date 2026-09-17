package com.litebalance.app.ui.home

import androidx.compose.animation.core.*
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.*
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.litebalance.app.core.CoreSession
import kotlinx.coroutines.launch
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.util.Locale
import kotlin.math.cos
import kotlin.math.sin

// ============================================================================
// 自然引力场：大地色系与生理潮汐隐喻 (Gravitational Tide - Organic Edition)
// 抛弃科技感，回归岩石、琥珀、水滴、草木的纯粹与静谧
// ============================================================================

val ColorLightBg = Color(0xFFF9F8F4) // 宣纸白/燕麦色
val ColorDarkBg = Color(0xFF23211E)  // 墨岩色/深木色

val ColorCoreLight = Color(0xFFD98A4F) // 琥珀色/窑烧陶土
val ColorCoreDark = Color(0xFFA85741)  // 赭石红

// 卫星色系 (自然萃取)
val ColorWater = Color(0xFF6B9080)   // 石青色/水滴
val ColorMeal = Color(0xFFE5C07B)    // 麦黄色/谷物
val ColorProtein = Color(0xFF708871) // 苔绿色/植物蛋白
val ColorExercise = Color(0xFFC87965) // 赤陶色/血液跳动

@Composable
fun HomeScreen(modifier: Modifier = Modifier) {
    val isDark = isSystemInDarkTheme()
    val bgColor = if (isDark) ColorDarkBg else ColorLightBg
    val coreColor = if (isDark) ColorCoreDark else ColorCoreLight

    // 状态拉取 (Mock 或真实)
    val isReady by CoreSession.isReady.collectAsState()
    var netBalanceKcal by remember { mutableDoubleStateOf(-596.0) }
    var waterMl by remember { mutableIntStateOf(1750) }
    val waterTargetMl = 2200

    LaunchedEffect(isReady) {
        if (isReady) {
            val today = LocalDate.now().toString()
            CoreSession.getEnergyBalance(today).onSuccess { b ->
                netBalanceKcal = b.netBalanceKcal
            }
            CoreSession.getDailyWater(today).onSuccess { w ->
                waterMl = w.totalConsumedMl.toInt()
            }
        }
    }

    Box(
        modifier = modifier
            .fillMaxSize()
            .background(bgColor)
            .statusBarsPadding()
    ) {
        Column(
            modifier = Modifier.fillMaxSize(),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            // 1. 顶层极简纪年 (取代枯燥的进度条与胶囊)
            OrganicDateHeader(isDark)

            Spacer(modifier = Modifier.weight(1f))

            // 2. 核心引力场视觉装置
            GravityField(
                coreColor = coreColor,
                waterProgress = waterMl.toFloat() / waterTargetMl.coerceAtLeast(1).toFloat(),
                isDark = isDark,
                modifier = Modifier
                    .fillMaxWidth()
                    .aspectRatio(1f)
                    .padding(32.dp)
            )

            Spacer(modifier = Modifier.weight(1f))

            // 3. 底层留白说明文字 (枯山水留白排版)
            OrganicFooter(isDark)
            Spacer(modifier = Modifier.height(64.dp))
        }
    }
}

@Composable
private fun OrganicDateHeader(isDark: Boolean) {
    val today = remember {
        val now = LocalDate.now()
        // 使用更具东方诗意的时间表达
        DateTimeFormatter.ofPattern("M月d日  ·  午时", Locale.CHINESE).format(now)
    }
    Text(
        text = today,
        style = MaterialTheme.typography.titleMedium.copy(
            fontWeight = FontWeight.Normal,
            letterSpacing = 3.sp,
            fontSize = 15.sp
        ),
        color = if (isDark) Color(0xFFD4CFC9) else Color(0xFF5C5750),
        modifier = Modifier.padding(top = 32.dp)
    )
}

@Composable
private fun GravityField(
    coreColor: Color,
    waterProgress: Float,
    isDark: Boolean,
    modifier: Modifier = Modifier
) {
    // 基础潮汐呼吸动画
    val infiniteTransition = rememberInfiniteTransition(label = "tide_breathing")
    val coreRadiusAnim by infiniteTransition.animateFloat(
        initialValue = 0.95f,
        targetValue = 1.08f, // 克制的形变，像呼吸
        animationSpec = infiniteRepeatable(
            animation = tween(4500, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse
        ),
        label = "core_radius"
    )

    // 星体公转轨道动画 (非常缓慢，犹如时间的流逝)
    val angleAnim by infiniteTransition.animateFloat(
        initialValue = 0f,
        targetValue = 360f,
        animationSpec = infiniteRepeatable(
            animation = tween(32000, easing = LinearEasing),
            repeatMode = RepeatMode.Restart
        ),
        label = "orbit_angle"
    )

    Canvas(modifier = modifier) {
        val center = Offset(size.width / 2, size.height / 2)
        val baseRadius = size.width / 5
        
        // 绘制静谧的枯山水沙盘轨道 (极其微弱的线条，不抢夺注意力)
        val trackColor = if (isDark) Color.White.copy(alpha = 0.04f) else Color.Black.copy(alpha = 0.03f)
        drawCircle(
            color = trackColor,
            radius = baseRadius * 1.6f,
            center = center,
            style = Stroke(width = 1.dp.toPx())
        )
        drawCircle(
            color = trackColor,
            radius = baseRadius * 2.3f,
            center = center,
            style = Stroke(width = 1.dp.toPx())
        )

        // 绘制中心代谢核 (温暖的琥珀/赭石体)
        // 1层：微弱的光晕
        drawCircle(
            color = coreColor.copy(alpha = 0.08f),
            radius = baseRadius * coreRadiusAnim * 1.4f,
            center = center
        )
        // 2层：本体
        drawCircle(
            color = coreColor.copy(alpha = 0.9f),
            radius = baseRadius * coreRadiusAnim,
            center = center
        )

        // ===== 绘制有机星体任务 =====

        // 1. 水滴 (在较近的轨道，代表高频需求)
        val waterRadius = baseRadius * 1.6f
        val waterAngleRad = Math.toRadians((angleAnim).toDouble())
        val waterX = center.x + waterRadius * cos(waterAngleRad).toFloat()
        val waterY = center.y + waterRadius * sin(waterAngleRad).toFloat()
        
        drawCircle(
            color = ColorWater,
            radius = 11.dp.toPx() + (3.dp.toPx() * coreRadiusAnim), // 随中心一起微弱呼吸
            center = Offset(waterX, waterY)
        )

        // 2. 谷物/午餐 (当前优先级最高，正被引力拉向中心，模拟脱离轨道的状态)
        // 午餐时间，它停留在两个轨道之间
        val mealRadius = baseRadius * 1.95f
        val mealAngleRad = Math.toRadians((-angleAnim * 0.8 + 140f).toDouble())
        val mealX = center.x + mealRadius * cos(mealAngleRad).toFloat()
        val mealY = center.y + mealRadius * sin(mealAngleRad).toFloat()
        
        drawCircle(
            color = ColorMeal,
            radius = 16.dp.toPx(),
            center = Offset(mealX, mealY)
        )
        // 午餐的引力牵引尾迹/高亮
        drawCircle(
            color = ColorMeal.copy(alpha = 0.3f),
            radius = 22.dp.toPx() * coreRadiusAnim,
            center = Offset(mealX, mealY)
        )

        // 3. 运动/赤陶 (在最外层轨道，代表下午/晚间的任务)
        val exerciseRadius = baseRadius * 2.3f
        val exAngleRad = Math.toRadians((angleAnim * 0.6 + 280f).toDouble())
        val exX = center.x + exerciseRadius * cos(exAngleRad).toFloat()
        val exY = center.y + exerciseRadius * sin(exAngleRad).toFloat()

        drawCircle(
            color = ColorExercise.copy(alpha = 0.8f),
            radius = 9.dp.toPx(),
            center = Offset(exX, exY)
        )
        
        // 4. 苔藓 (可能是维他命或轻量习惯)
        val mossRadius = baseRadius * 2.3f
        val mossAngleRad = Math.toRadians((angleAnim * 0.6 + 50f).toDouble())
        val mossX = center.x + mossRadius * cos(mossAngleRad).toFloat()
        val mossY = center.y + mossRadius * sin(mossAngleRad).toFloat()

        drawCircle(
            color = ColorProtein.copy(alpha = 0.7f),
            radius = 7.dp.toPx(),
            center = Offset(mossX, mossY)
        )
    }
}

@Composable
private fun OrganicFooter(isDark: Boolean) {
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Text(
            text = "宜 补 充 优 质 蛋 白",
            style = MaterialTheme.typography.bodyLarge.copy(
                fontWeight = FontWeight.Medium,
                letterSpacing = 4.sp,
                fontSize = 15.sp
            ),
            color = if (isDark) Color(0xFFD4CFC9) else Color(0xFF4A453F)
        )
        Spacer(modifier = Modifier.height(14.dp))
        Text(
            text = "午餐的引力正在增强",
            style = MaterialTheme.typography.bodySmall.copy(
                letterSpacing = 1.sp,
                fontSize = 11.sp
            ),
            color = if (isDark) Color(0xFF7A756D) else Color(0xFF9E9A93)
        )
    }
}
