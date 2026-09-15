package com.litebalance.app.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/**
 * 苹果 iPhone「液态玻璃 (Liquid Glass)」设计语言
 * 汲取 iOS 18 与 Apple 晶透流体材质灵感：
 * - 饱满晶透感 (Luminous Translucency)
 * - 顶部边缘折射高光 (Specular Top Highlight)
 * - 漫反射柔晕 (Soft Ambient Glow)
 * - 水滴有机温润感，坚决摒弃冷冰冰的科技感
 */
object AppleColors {
    // 背景与表面基调
    val CanvasLight = Color(0xFFF6F7F9)        // 浅暖晨雾背景，接近 iOS 系统
    val SurfaceLight = Color(0xFFFFFFFF)
    val SurfaceSubtleLight = Color(0xFFF1F3F6)
    val BorderLight = Color(0xFFE4E7EB)

    val CanvasDark = Color(0xFF0F1013)         // 石板黑底
    val SurfaceDark = Color(0xFF1B1D22)
    val SurfaceSubtleDark = Color(0xFF242730)
    val BorderDark = Color(0xFF2E323B)

    // 文字阶梯
    val LabelPrimaryLight = Color(0xFF1A1C1E)
    val LabelSecondaryLight = Color(0xFF6B7280)
    val LabelTertiaryLight = Color(0xFF9CA3AF)

    val LabelPrimaryDark = Color(0xFFF3F4F6)
    val LabelSecondaryDark = Color(0xFF9CA3AF)
    val LabelTertiaryDark = Color(0xFF6B7280)

    // 经典 Apple Health 功能代表色 (温和、高识别度、人文感)
    val VitalityCoral = Color(0xFFFF453A)      // 活力桃红 / 能量与活动闭环
    val VitalityCoralSoft = Color(0xFFFFECEB)
    val VitalityCoralGlass = Color(0x33FF453A)

    val MintGreen = Color(0xFF34C759)          // 鼠尾草绿 / 健康蛋白质与达标
    val MintGreenSoft = Color(0xFFEBF9EE)
    val MintGreenGlass = Color(0x3334C759)

    val AmberGold = Color(0xFFFF9F0A)          // 温暖杏橙 / 碳水与间歇性断食
    val AmberGoldSoft = Color(0xFFFFF6E8)
    val AmberGoldGlass = Color(0x33FF9F0A)

    val CalmIndigo = Color(0xFF007AFF)         // 宁静微蓝 / 水分与体脂
    val CalmIndigoSoft = Color(0xFFEAF4FF)
    val CalmIndigoGlass = Color(0x33007AFF)

    val IrisPurple = Color(0xFFAF52DE)         // 柔和鸢尾紫 / 代谢仿真与AI洞察
    val IrisPurpleSoft = Color(0xFFF7ECFC)
    val IrisPurpleGlass = Color(0x33AF52DE)

    val SlatePebble = Color(0xFF8E8E93)
}

/**
 * 苹果 iPhone 液态玻璃 (Liquid Glass) 材质系统
 */
object LiquidGlassTokens {
    // 浅色模式液态玻璃底色渐变 (微通透、晶润高光)
    val LightBody = Brush.verticalGradient(
        colors = listOf(
            Color(0xFAFFFFFF), // 顶部微光更透亮
            Color(0xEEF8FAFC), // 底部微温润
        ),
    )

    // 浅色模式液态玻璃高光反射边线 (上方强反射，下方弱漫射)
    val LightBorder = Brush.verticalGradient(
        colors = listOf(
            Color(0xFFFFFFFF),       // 顶部镜面反光 (Specular Ridge)
            Color(0x66E2E8F0),       // 中段过渡
            Color(0x22CBD5E1),       // 底部弱消散
        ),
    )

    // 浅色模式微光水滴背景 (用于微件与高亮卡片)
    val CoralLiquidGlow = Brush.verticalGradient(
        colors = listOf(
            Color(0x24FF453A),
            Color(0x0CFF453A),
        ),
    )

    val MintLiquidGlow = Brush.verticalGradient(
        colors = listOf(
            Color(0x2434C759),
            Color(0x0C34C759),
        ),
    )

    val AmberLiquidGlow = Brush.verticalGradient(
        colors = listOf(
            Color(0x24FF9F0A),
            Color(0x0CFF9F0A),
        ),
    )

    val IndigoLiquidGlow = Brush.verticalGradient(
        colors = listOf(
            Color(0x24007AFF),
            Color(0x0C007AFF),
        ),
    )

    // 深色模式液态黑曜玻璃
    val DarkBody = Brush.verticalGradient(
        colors = listOf(
            Color(0xD9252830),
            Color(0xC01A1C22),
        ),
    )

    val DarkBorder = Brush.verticalGradient(
        colors = listOf(
            Color(0x55FFFFFF),
            Color(0x1F94A3B8),
            Color(0x0A000000),
        ),
    )
}

val AppleShapes = Shapes(
    extraSmall = RoundedCornerShape(10.dp),
    small = RoundedCornerShape(14.dp),
    medium = RoundedCornerShape(20.dp),
    large = RoundedCornerShape(26.dp),        // 苹果液态玻璃标志性大圆角
    extraLarge = RoundedCornerShape(34.dp),
)

val AppleTypography = Typography(
    displayLarge = TextStyle(
        fontWeight = FontWeight.Bold,
        fontSize = 34.sp,
        lineHeight = 41.sp,
        letterSpacing = 0.37.sp,
    ),
    headlineMedium = TextStyle(
        fontWeight = FontWeight.Bold,
        fontSize = 24.sp,
        lineHeight = 30.sp,
        letterSpacing = 0.35.sp,
    ),
    headlineSmall = TextStyle(
        fontWeight = FontWeight.SemiBold,
        fontSize = 20.sp,
        lineHeight = 26.sp,
        letterSpacing = 0.38.sp,
    ),
    titleLarge = TextStyle(
        fontWeight = FontWeight.SemiBold,
        fontSize = 18.sp,
        lineHeight = 24.sp,
        letterSpacing = 0.sp,
    ),
    titleMedium = TextStyle(
        fontWeight = FontWeight.Medium,
        fontSize = 16.sp,
        lineHeight = 22.sp,
        letterSpacing = 0.15.sp,
    ),
    bodyLarge = TextStyle(
        fontWeight = FontWeight.Normal,
        fontSize = 16.sp,
        lineHeight = 22.sp,
        letterSpacing = 0.25.sp,
    ),
    bodyMedium = TextStyle(
        fontWeight = FontWeight.Normal,
        fontSize = 14.sp,
        lineHeight = 20.sp,
        letterSpacing = 0.1.sp,
    ),
    labelLarge = TextStyle(
        fontWeight = FontWeight.SemiBold,
        fontSize = 13.sp,
        lineHeight = 18.sp,
        letterSpacing = 0.1.sp,
    ),
    labelMedium = TextStyle(
        fontWeight = FontWeight.Medium,
        fontSize = 12.sp,
        lineHeight = 16.sp,
        letterSpacing = 0.2.sp,
    ),
    labelSmall = TextStyle(
        fontWeight = FontWeight.Normal,
        fontSize = 11.sp,
        lineHeight = 14.sp,
        letterSpacing = 0.2.sp,
    ),
)

private val AppleLightScheme = lightColorScheme(
    primary = AppleColors.VitalityCoral,
    onPrimary = Color.White,
    primaryContainer = AppleColors.VitalityCoralSoft,
    onPrimaryContainer = AppleColors.VitalityCoral,
    secondary = AppleColors.MintGreen,
    onSecondary = Color.White,
    secondaryContainer = AppleColors.MintGreenSoft,
    onSecondaryContainer = AppleColors.MintGreen,
    tertiary = AppleColors.AmberGold,
    background = AppleColors.CanvasLight,
    onBackground = AppleColors.LabelPrimaryLight,
    surface = AppleColors.SurfaceLight,
    onSurface = AppleColors.LabelPrimaryLight,
    surfaceVariant = AppleColors.SurfaceSubtleLight,
    onSurfaceVariant = AppleColors.LabelSecondaryLight,
    outline = AppleColors.BorderLight,
)

private val AppleDarkScheme = darkColorScheme(
    primary = AppleColors.VitalityCoral,
    onPrimary = Color.White,
    primaryContainer = Color(0xFF3F1918),
    onPrimaryContainer = Color(0xFFFFB4AB),
    secondary = AppleColors.MintGreen,
    onSecondary = Color.White,
    secondaryContainer = Color(0xFF12381A),
    onSecondaryContainer = Color(0xFF90F7A6),
    tertiary = AppleColors.AmberGold,
    background = AppleColors.CanvasDark,
    onBackground = AppleColors.LabelPrimaryDark,
    surface = AppleColors.SurfaceDark,
    onSurface = AppleColors.LabelPrimaryDark,
    surfaceVariant = AppleColors.SurfaceSubtleDark,
    onSurfaceVariant = AppleColors.LabelSecondaryDark,
    outline = AppleColors.BorderDark,
)

@Composable
fun ThemeLiteBalance(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit,
) {
    val colorScheme = if (darkTheme) AppleDarkScheme else AppleLightScheme
    MaterialTheme(
        colorScheme = colorScheme,
        shapes = AppleShapes,
        typography = AppleTypography,
        content = content,
    )
}
