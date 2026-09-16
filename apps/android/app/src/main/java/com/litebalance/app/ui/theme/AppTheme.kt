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

/** 应用配色。 */
object AppColors {
    val CanvasLight = Color(0xFFF6F7F9)
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

    val VitalityCoral = Color(0xFFFF453A)
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

    val IrisPurple = Color(0xFFAF52DE)
    val IrisPurpleSoft = Color(0xFFF7ECFC)
    val IrisPurpleGlass = Color(0x33AF52DE)

    val SlatePebble = Color(0xFF8E8E93)
}

object LiquidGlassTokens {
    val LightBody = Brush.verticalGradient(
        colors = listOf(
            Color(0xFAFFFFFF),
            Color(0xEEF8FAFC),
        ),
    )

    val LightBorder = Brush.verticalGradient(
        colors = listOf(
            Color(0xFFFFFFFF),
            Color(0x66E2E8F0),
            Color(0x22CBD5E1),
        ),
    )

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

val AppShapes = Shapes(
    extraSmall = RoundedCornerShape(10.dp),
    small = RoundedCornerShape(14.dp),
    medium = RoundedCornerShape(20.dp),
    large = RoundedCornerShape(26.dp),
    extraLarge = RoundedCornerShape(34.dp),
)

val AppTypography = Typography(
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

private val LightColorScheme = lightColorScheme(
    primary = AppColors.VitalityCoral,
    onPrimary = Color.White,
    primaryContainer = AppColors.VitalityCoralSoft,
    onPrimaryContainer = AppColors.VitalityCoral,
    secondary = AppColors.MintGreen,
    onSecondary = Color.White,
    secondaryContainer = AppColors.MintGreenSoft,
    onSecondaryContainer = AppColors.MintGreen,
    tertiary = AppColors.AmberGold,
    background = AppColors.CanvasLight,
    onBackground = AppColors.LabelPrimaryLight,
    surface = AppColors.SurfaceLight,
    onSurface = AppColors.LabelPrimaryLight,
    surfaceVariant = AppColors.SurfaceSubtleLight,
    onSurfaceVariant = AppColors.LabelSecondaryLight,
    outline = AppColors.BorderLight,
)

private val DarkColorScheme = darkColorScheme(
    primary = AppColors.VitalityCoral,
    onPrimary = Color.White,
    primaryContainer = Color(0xFF3F1918),
    onPrimaryContainer = Color(0xFFFFB4AB),
    secondary = AppColors.MintGreen,
    onSecondary = Color.White,
    secondaryContainer = Color(0xFF12381A),
    onSecondaryContainer = Color(0xFF90F7A6),
    tertiary = AppColors.AmberGold,
    background = AppColors.CanvasDark,
    onBackground = AppColors.LabelPrimaryDark,
    surface = AppColors.SurfaceDark,
    onSurface = AppColors.LabelPrimaryDark,
    surfaceVariant = AppColors.SurfaceSubtleDark,
    onSurfaceVariant = AppColors.LabelSecondaryDark,
    outline = AppColors.BorderDark,
)

@Composable
fun ThemeLiteBalance(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit,
) {
    val colorScheme = if (darkTheme) DarkColorScheme else LightColorScheme
    MaterialTheme(
        colorScheme = colorScheme,
        shapes = AppShapes,
        typography = AppTypography,
        content = content,
    )
}
