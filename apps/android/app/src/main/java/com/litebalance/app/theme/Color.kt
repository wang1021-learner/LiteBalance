package com.litebalance.app.theme

import androidx.compose.ui.graphics.Color

// ===========================================================================
// 品牌四色（整体 UI 以此为准）
// ===========================================================================
/** 柔和奶白 */
val BrandCream = Color(0xFFF9F7F2)
/** 柔和珊瑚粉 */
val BrandCoral = Color(0xFFE8B4B8)
/** 柔和薄荷绿 */
val BrandMint = Color(0xFFB5D5C5)
/** 柔和暖灰 */
val BrandWarmGray = Color(0xFFD4CFC9)

private val Ink = Color(0xFF2C322F)
private val InkSoft = Color(0xFF5C6560)

// --- Light Theme ---
val md_theme_light_primary = Color(0xFF4A7A66)
val md_theme_light_onPrimary = Color(0xFFFFFFFF)
val md_theme_light_primaryContainer = BrandMint
val md_theme_light_onPrimaryContainer = Color(0xFF1E332A)

val md_theme_light_secondary = Color(0xFF6E6A63)
val md_theme_light_onSecondary = Color(0xFFFFFFFF)
val md_theme_light_secondaryContainer = BrandWarmGray
val md_theme_light_onSecondaryContainer = Color(0xFF2F2C28)

val md_theme_light_tertiary = Color(0xFFB57A80)
val md_theme_light_onTertiary = Color(0xFFFFFFFF)
val md_theme_light_tertiaryContainer = BrandCoral
val md_theme_light_onTertiaryContainer = Color(0xFF4A2428)

val md_theme_light_error = Color(0xFFB54A4A)
val md_theme_light_errorContainer = Color(0xFFF3D4D4)
val md_theme_light_onError = Color(0xFFFFFFFF)
val md_theme_light_onErrorContainer = Color(0xFF4A1818)

val md_theme_light_background = BrandCream
val md_theme_light_onBackground = Ink
val md_theme_light_surface = BrandCream
val md_theme_light_onSurface = Ink
val md_theme_light_surfaceVariant = Color(0xFFECE8E1)
val md_theme_light_onSurfaceVariant = InkSoft
val md_theme_light_outline = BrandWarmGray
val md_theme_light_outlineVariant = Color(0xFFE2DDD6)

val md_theme_light_surfaceContainerLowest = Color(0xFFFFFFFF)
val md_theme_light_surfaceContainerLow = Color(0xFFFCFBF8)
val md_theme_light_surfaceContainer = Color(0xFFF3F0EA)
val md_theme_light_surfaceContainerHigh = Color(0xFFEDE9E2)
val md_theme_light_surfaceContainerHighest = Color(0xFFE6E2DA)

// --- Dark Theme ---
val md_theme_dark_primary = BrandMint
val md_theme_dark_onPrimary = Color(0xFF1A2E25)
val md_theme_dark_primaryContainer = Color(0xFF4A7A66)
val md_theme_dark_onPrimaryContainer = Color(0xFFE4F0EA)

val md_theme_dark_secondary = BrandWarmGray
val md_theme_dark_onSecondary = Color(0xFF2A2824)
val md_theme_dark_secondaryContainer = Color(0xFF4A4742)
val md_theme_dark_onSecondaryContainer = Color(0xFFE8E4DC)

val md_theme_dark_tertiary = BrandCoral
val md_theme_dark_onTertiary = Color(0xFF3A1C1C)
val md_theme_dark_tertiaryContainer = Color(0xFF7A4548)
val md_theme_dark_onTertiaryContainer = Color(0xFFFFE8EA)

val md_theme_dark_error = Color(0xFFE8B4B8)
val md_theme_dark_errorContainer = Color(0xFF7A3030)
val md_theme_dark_onError = Color(0xFF3A1010)
val md_theme_dark_onErrorContainer = Color(0xFFFFDAD6)

val md_theme_dark_background = Color(0xFF1A1B19)
val md_theme_dark_onBackground = Color(0xFFE8E4DC)
val md_theme_dark_surface = Color(0xFF1A1B19)
val md_theme_dark_onSurface = Color(0xFFE8E4DC)
val md_theme_dark_surfaceVariant = Color(0xFF3A3C38)
val md_theme_dark_onSurfaceVariant = Color(0xFFD4CFC9)
val md_theme_dark_outline = Color(0xFF8A8680)
val md_theme_dark_outlineVariant = Color(0xFF3A3C38)

val md_theme_dark_surfaceContainerLowest = Color(0xFF121311)
val md_theme_dark_surfaceContainerLow = Color(0xFF222421)
val md_theme_dark_surfaceContainer = Color(0xFF262824)
val md_theme_dark_surfaceContainerHigh = Color(0xFF31332F)
val md_theme_dark_surfaceContainerHighest = Color(0xFF3C3E3A)

// 语义色（四色家族内）
val MacroProteinColor = Color(0xFF4A7A66)
val MacroCarbColor = Color(0xFFC47A80)
val MacroFatColor = Color(0xFFB8A878)
val WaterHydrationColor = Color(0xFF6A9B8C)
val FastingPurpleColor = Color(0xFF8A7A78)

val CaloricDeficitColor = Color(0xFF4A7A66)
val CaloricSurplusColor = Color(0xFFB57A80)
val CaloricDangerColor = Color(0xFFA04545)
