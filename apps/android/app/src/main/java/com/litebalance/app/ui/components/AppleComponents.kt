package com.litebalance.app.ui.components

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.AutoAwesome
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.litebalance.app.ui.theme.AppleColors
import com.litebalance.app.ui.theme.LiquidGlassTokens

/**
 * 苹果 iPhone「液态玻璃 (Liquid Glass)」卡片
 * 具备三大物理级光学特征：
 * 1. 晶润微透光材质底色 (Translucent Fluid Body)
 * 2. 边缘顶部折射镜面高光 (Specular Ridge Light)
 * 3. 柔和有机漫晕 (Organic Ambient Soft Shadow)
 */
@Composable
fun AppleCard(
    modifier: Modifier = Modifier,
    customGlowBrush: Brush? = null,
    backgroundColor: Color? = null,
    contentPadding: Dp = 18.dp,
    onClick: (() -> Unit)? = null,
    content: @Composable ColumnScopeWrapper.() -> Unit,
) {
    val isDark = isSystemInDarkTheme()
    val shape = RoundedCornerShape(24.dp)

    // 镜面折射光渐变边框：顶部强反射白光，底部微弱消散
    val specularBorder = if (isDark) LiquidGlassTokens.DarkBorder else LiquidGlassTokens.LightBorder

    val cardModifier = if (onClick != null) {
        modifier
            .clip(shape)
            .clickable(onClick = onClick)
    } else {
        modifier
    }

    Box(
        modifier = cardModifier
            .fillMaxWidth()
            // 苹果液态玻璃极柔漫晕
            .shadow(
                elevation = 6.dp,
                shape = shape,
                ambientColor = Color(0x14000000),
                spotColor = Color(0x0F000000),
            )
            .clip(shape)
            .background(
                brush = customGlowBrush ?: if (backgroundColor != null) {
                    Brush.verticalGradient(listOf(backgroundColor, backgroundColor.copy(alpha = 0.88f)))
                } else if (isDark) {
                    LiquidGlassTokens.DarkBody
                } else {
                    LiquidGlassTokens.LightBody
                },
            )
            // 边缘全反射高光
            .border(width = 1.dp, brush = specularBorder, shape = shape)
            .padding(contentPadding),
    ) {
        Column(
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            ColumnScopeWrapper.content()
        }
    }
}

object ColumnScopeWrapper

/**
 * 苹果 iPhone 液态玻璃活动闭环 (Liquid Activity Rings)
 * 模拟高饱和度流体光柱闭环，具有晶莹透光的环槽底色。
 */
@Composable
fun AppleActivityRings(
    modifier: Modifier = Modifier,
    intakeProgress: Float,
    exerciseProgress: Float,
    waterProgress: Float,
    size: Dp = 142.dp,
    strokeWidth: Dp = 12.dp,
) {
    val animIntake by animateFloatAsState(
        targetValue = intakeProgress.coerceIn(0f, 1.5f),
        animationSpec = tween(durationMillis = 800, easing = FastOutSlowInEasing),
        label = "animIntake",
    )
    val animExercise by animateFloatAsState(
        targetValue = exerciseProgress.coerceIn(0f, 1.5f),
        animationSpec = tween(durationMillis = 850, easing = FastOutSlowInEasing),
        label = "animExercise",
    )
    val animWater by animateFloatAsState(
        targetValue = waterProgress.coerceIn(0f, 1.5f),
        animationSpec = tween(durationMillis = 900, easing = FastOutSlowInEasing),
        label = "animWater",
    )

    Box(
        modifier = modifier.size(size),
        contentAlignment = Alignment.Center,
    ) {
        Canvas(modifier = Modifier.size(size)) {
            val strokePx = strokeWidth.toPx()
            val gapPx = strokePx * 0.35f
            val centerOffset = Offset(this.size.width / 2, this.size.height / 2)

            // 1. 外环：活力桃红 (热量摄入)
            val r1 = (this.size.width - strokePx) / 2
            // 晶透水润凹槽
            drawCircle(
                color = AppleColors.VitalityCoral.copy(alpha = 0.16f),
                radius = r1,
                center = centerOffset,
                style = Stroke(width = strokePx),
            )
            // 流体光柱
            drawArc(
                brush = Brush.sweepGradient(
                    listOf(
                        AppleColors.VitalityCoral.copy(alpha = 0.85f),
                        AppleColors.VitalityCoral,
                    ),
                    center = centerOffset,
                ),
                startAngle = -90f,
                sweepAngle = animIntake * 360f,
                useCenter = false,
                topLeft = Offset(centerOffset.x - r1, centerOffset.y - r1),
                size = Size(r1 * 2, r1 * 2),
                style = Stroke(width = strokePx, cap = StrokeCap.Round),
            )

            // 2. 中环：鼠尾草绿 (运动净消耗)
            val r2 = r1 - strokePx - gapPx
            if (r2 > 0) {
                drawCircle(
                    color = AppleColors.MintGreen.copy(alpha = 0.16f),
                    radius = r2,
                    center = centerOffset,
                    style = Stroke(width = strokePx),
                )
                drawArc(
                    brush = Brush.sweepGradient(
                        listOf(
                            AppleColors.MintGreen.copy(alpha = 0.85f),
                            AppleColors.MintGreen,
                        ),
                        center = centerOffset,
                    ),
                    startAngle = -90f,
                    sweepAngle = animExercise * 360f,
                    useCenter = false,
                    topLeft = Offset(centerOffset.x - r2, centerOffset.y - r2),
                    size = Size(r2 * 2, r2 * 2),
                    style = Stroke(width = strokePx, cap = StrokeCap.Round),
                )
            }

            // 3. 内环：宁静微蓝 (补水)
            val r3 = r2 - strokePx - gapPx
            if (r3 > 0) {
                drawCircle(
                    color = AppleColors.CalmIndigo.copy(alpha = 0.16f),
                    radius = r3,
                    center = centerOffset,
                    style = Stroke(width = strokePx),
                )
                drawArc(
                    brush = Brush.sweepGradient(
                        listOf(
                            AppleColors.CalmIndigo.copy(alpha = 0.85f),
                            AppleColors.CalmIndigo,
                        ),
                        center = centerOffset,
                    ),
                    startAngle = -90f,
                    sweepAngle = animWater * 360f,
                    useCenter = false,
                    topLeft = Offset(centerOffset.x - r3, centerOffset.y - r3),
                    size = Size(r3 * 2, r3 * 2),
                    style = Stroke(width = strokePx, cap = StrokeCap.Round),
                )
            }
        }
    }
}

/**
 * 苹果 iPhone 液态玻璃水滴徽章 (Liquid Glass Capsule Badge)
 */
@Composable
fun CapsuleBadge(
    text: String,
    modifier: Modifier = Modifier,
    color: Color = AppleColors.MintGreen,
    backgroundColor: Color = color.copy(alpha = 0.12f),
) {
    Box(
        modifier = modifier
            .clip(CircleShape)
            .background(
                Brush.verticalGradient(
                    listOf(
                        backgroundColor,
                        backgroundColor.copy(alpha = 0.05f),
                    ),
                ),
            )
            .border(
                width = 0.8.dp,
                brush = Brush.verticalGradient(
                    listOf(
                        Color.White.copy(alpha = 0.7f),
                        color.copy(alpha = 0.25f),
                    ),
                ),
                shape = CircleShape,
            )
            .padding(horizontal = 10.dp, vertical = 4.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = text,
            style = MaterialTheme.typography.labelMedium.copy(
                fontWeight = FontWeight.SemiBold,
                color = color,
                fontSize = 11.5.sp,
            ),
        )
    }
}

/**
 * 苹果 iPhone 液态玻璃药丸分段选择器 (Liquid Segmented Control)
 */
@Composable
fun AppleSegmentedControl(
    items: List<String>,
    selectedIndex: Int,
    onSelect: (Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    val isDark = isSystemInDarkTheme()
    val troughShape = RoundedCornerShape(16.dp)

    Row(
        modifier = modifier
            .fillMaxWidth()
            .clip(troughShape)
            // 半透明流体凹槽
            .background(
                if (isDark) Color(0x66242730) else Color(0x33E2E8F0),
            )
            .border(
                0.8.dp,
                Brush.verticalGradient(
                    listOf(
                        Color.White.copy(alpha = 0.35f),
                        Color.Transparent,
                    ),
                ),
                troughShape,
            )
            .padding(3.5.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        items.forEachIndexed { index, title ->
            val isSelected = index == selectedIndex
            val pillShape = RoundedCornerShape(12.dp)

            val textWeight = if (isSelected) FontWeight.Bold else FontWeight.Medium
            val textColor by animateColorAsState(
                targetValue = if (isSelected) MaterialTheme.colorScheme.onSurface else MaterialTheme.colorScheme.onSurfaceVariant,
                label = "segTextColor",
            )

            Box(
                modifier = Modifier
                    .weight(1f)
                    .clip(pillShape)
                    .then(
                        if (isSelected) {
                            Modifier
                                // 液态玻璃悬浮药丸：纯白微光 + 顶部镜面反光
                                .shadow(4.dp, pillShape, ambientColor = Color(0x1A000000))
                                .background(
                                    Brush.verticalGradient(
                                        listOf(
                                            Color(0xFFFFFFFF),
                                            Color(0xFFF9FAFB),
                                        ),
                                    ),
                                )
                                .border(
                                    width = 0.8.dp,
                                    brush = Brush.verticalGradient(
                                        listOf(
                                            Color(0xFFFFFFFF),
                                            Color(0x66CBD5E1),
                                        ),
                                    ),
                                    shape = pillShape,
                                )
                        } else {
                            Modifier
                        },
                    )
                    .clickable(
                        interactionSource = remember { MutableInteractionSource() },
                        indication = null,
                    ) { onSelect(index) }
                    .padding(vertical = 8.dp),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    text = title,
                    style = MaterialTheme.typography.labelMedium.copy(
                        fontWeight = textWeight,
                        color = textColor,
                        fontSize = 12.5.sp,
                    ),
                )
            }
        }
    }
}

/**
 * 提示卡片（默认标注为「演示规则」，避免被误解为真实 AI 模型）。
 */
@Composable
fun AiInsightCard(
    title: String,
    insight: String,
    modifier: Modifier = Modifier,
    badgeText: String = "演示规则",
    actionLabel: String? = null,
    onAction: (() -> Unit)? = null,
) {
    val shape = RoundedCornerShape(24.dp)
    val fluidBorder = Brush.sweepGradient(
        listOf(
            AppleColors.IrisPurple.copy(alpha = 0.65f),
            AppleColors.VitalityCoral.copy(alpha = 0.5f),
            AppleColors.AmberGold.copy(alpha = 0.45f),
            AppleColors.CalmIndigo.copy(alpha = 0.5f),
            AppleColors.IrisPurple.copy(alpha = 0.65f),
        ),
    )

    Box(
        modifier = modifier
            .fillMaxWidth()
            .shadow(
                elevation = 8.dp,
                shape = shape,
                ambientColor = AppleColors.IrisPurple.copy(alpha = 0.18f),
                spotColor = AppleColors.VitalityCoral.copy(alpha = 0.12f),
            )
            .clip(shape)
            // 半透明水润晶晶紫
            .background(
                Brush.verticalGradient(
                    listOf(
                        AppleColors.IrisPurpleSoft.copy(alpha = 0.82f),
                        AppleColors.SurfaceLight.copy(alpha = 0.95f),
                    ),
                ),
            )
            .border(width = 1.2.dp, brush = fluidBorder, shape = shape)
            .padding(18.dp),
    ) {
        Column(
            verticalArrangement = Arrangement.spacedBy(9.dp),
        ) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Box(
                    modifier = Modifier
                        .size(30.dp)
                        .clip(CircleShape)
                        .background(
                            Brush.radialGradient(
                                listOf(
                                    AppleColors.IrisPurple.copy(alpha = 0.28f),
                                    AppleColors.IrisPurple.copy(alpha = 0.08f),
                                ),
                            ),
                        )
                        .border(
                            0.8.dp,
                            Brush.verticalGradient(
                                listOf(
                                    Color.White.copy(alpha = 0.8f),
                                    AppleColors.IrisPurple.copy(alpha = 0.3f),
                                ),
                            ),
                            CircleShape,
                        ),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        imageVector = Icons.Rounded.AutoAwesome,
                        contentDescription = badgeText,
                        tint = AppleColors.IrisPurple,
                        modifier = Modifier.size(16.dp),
                    )
                }
                Text(
                    text = title,
                    style = MaterialTheme.typography.titleMedium.copy(
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onSurface,
                    ),
                )
                Spacer(modifier = Modifier.weight(1f))
                CapsuleBadge(
                    text = badgeText,
                    color = AppleColors.IrisPurple,
                    backgroundColor = AppleColors.IrisPurple.copy(alpha = 0.15f),
                )
            }

            Text(
                text = insight,
                style = MaterialTheme.typography.bodyMedium.copy(
                    lineHeight = 22.sp,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                ),
            )

            if (actionLabel != null && onAction != null) {
                Spacer(modifier = Modifier.height(2.dp))
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.End,
                ) {
                    Text(
                        text = actionLabel,
                        style = MaterialTheme.typography.labelLarge.copy(
                            color = AppleColors.IrisPurple,
                            fontWeight = FontWeight.Bold,
                        ),
                        modifier = Modifier
                            .clip(RoundedCornerShape(8.dp))
                            .clickable(onClick = onAction)
                            .padding(horizontal = 8.dp, vertical = 4.dp),
                    )
                }
            }
        }
    }
}

/**
 * 苹果 iPhone 经典数据指标元 (Apple Metric Cell)
 */
@Composable
fun AppleMetricItem(
    label: String,
    value: String,
    unit: String = "",
    sublabel: String? = null,
    accentColor: Color = MaterialTheme.colorScheme.onSurface,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier = modifier,
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.labelMedium.copy(
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            ),
        )
        Row(
            verticalAlignment = Alignment.Bottom,
            horizontalArrangement = Arrangement.spacedBy(3.dp),
        ) {
            Text(
                text = value,
                style = MaterialTheme.typography.headlineMedium.copy(
                    fontWeight = FontWeight.Bold,
                    color = accentColor,
                ),
            )
            if (unit.isNotBlank()) {
                Text(
                    text = unit,
                    style = MaterialTheme.typography.labelMedium.copy(
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        fontWeight = FontWeight.Medium,
                    ),
                    modifier = Modifier.padding(bottom = 3.5.dp),
                )
            }
        }
        if (sublabel != null) {
            Text(
                text = sublabel,
                style = MaterialTheme.typography.labelSmall.copy(
                    color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.8f),
                ),
            )
        }
    }
}

/**
 * 苹果 iPhone 液态晶透输入框 (Liquid TextField)
 */
@Composable
fun AppleTextField(
    value: String,
    onValueChange: (String) -> Unit,
    label: String,
    modifier: Modifier = Modifier,
    placeholder: String = "",
    singleLine: Boolean = true,
    trailingText: String? = null,
) {
    val isDark = isSystemInDarkTheme()
    val shape = RoundedCornerShape(16.dp)

    OutlinedTextField(
        value = value,
        onValueChange = onValueChange,
        label = { Text(label) },
        placeholder = if (placeholder.isNotBlank()) { { Text(placeholder) } } else null,
        trailingIcon = if (trailingText != null) {
            {
                Text(
                    text = trailingText,
                    style = MaterialTheme.typography.labelMedium.copy(
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    ),
                    modifier = Modifier.padding(end = 12.dp),
                )
            }
        } else null,
        singleLine = singleLine,
        shape = shape,
        colors = TextFieldDefaults.colors(
            focusedContainerColor = if (isDark) Color(0x55252830) else Color(0xD9FFFFFF),
            unfocusedContainerColor = if (isDark) Color(0x33252830) else Color(0x99F1F3F6),
            focusedIndicatorColor = AppleColors.VitalityCoral,
            unfocusedIndicatorColor = Color(0x22CBD5E1),
        ),
        modifier = modifier
            .fillMaxWidth()
            .border(
                0.8.dp,
                Brush.verticalGradient(
                    listOf(
                        Color.White.copy(alpha = 0.6f),
                        Color.Transparent,
                    ),
                ),
                shape,
            ),
    )
}

/**
 * 苹果 iPhone 液态水滴胶囊按钮 (Liquid Pill Button)
 */
@Composable
fun AppleButton(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    color: Color = AppleColors.VitalityCoral,
    textColor: Color = Color.White,
    outlined: Boolean = false,
) {
    if (outlined) {
        Box(
            modifier = modifier
                .clip(CircleShape)
                .border(
                    width = 1.2.dp,
                    brush = Brush.verticalGradient(
                        listOf(
                            color.copy(alpha = 0.9f),
                            color.copy(alpha = 0.4f),
                        ),
                    ),
                    shape = CircleShape,
                )
                .background(
                    Brush.verticalGradient(
                        listOf(
                            color.copy(alpha = 0.08f),
                            Color.Transparent,
                        ),
                    ),
                )
                .clickable(onClick = onClick)
                .padding(horizontal = 20.dp, vertical = 11.dp),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = text,
                style = MaterialTheme.typography.labelLarge.copy(
                    color = color,
                    fontWeight = FontWeight.SemiBold,
                ),
            )
        }
    } else {
        // 实心水滴液态按钮：顶部微高光折射 + 饱满有机色泽
        Box(
            modifier = modifier
                .shadow(
                    elevation = 6.dp,
                    shape = CircleShape,
                    ambientColor = color.copy(alpha = 0.3f),
                    spotColor = color.copy(alpha = 0.25f),
                )
                .clip(CircleShape)
                .background(
                    Brush.verticalGradient(
                        listOf(
                            color.copy(alpha = 0.96f),
                            color,
                        ),
                    ),
                )
                .border(
                    width = 1.dp,
                    brush = Brush.verticalGradient(
                        listOf(
                            Color.White.copy(alpha = 0.45f),
                            Color.Transparent,
                        ),
                    ),
                    shape = CircleShape,
                )
                .clickable(onClick = onClick)
                .padding(horizontal = 22.dp, vertical = 12.dp),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = text,
                style = MaterialTheme.typography.labelLarge.copy(
                    color = textColor,
                    fontWeight = FontWeight.Bold,
                    fontSize = 15.sp,
                ),
            )
        }
    }
}
