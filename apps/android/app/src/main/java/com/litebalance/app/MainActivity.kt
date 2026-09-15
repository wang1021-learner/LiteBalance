package com.litebalance.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.DirectionsRun
import androidx.compose.material.icons.rounded.GridView
import androidx.compose.material.icons.rounded.Insights
import androidx.compose.material.icons.rounded.Restaurant
import androidx.compose.material.icons.rounded.WbSunny
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import com.litebalance.app.ui.ActivityScreen
import com.litebalance.app.ui.DashboardScreen
import com.litebalance.app.ui.FoodScreen
import com.litebalance.app.ui.HealthHubScreen
import com.litebalance.app.ui.PlanScreen
import com.litebalance.app.ui.theme.AppleColors
import com.litebalance.app.ui.theme.LiquidGlassTokens
import com.litebalance.app.ui.theme.ThemeLiteBalance

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        // 预热并打开原生 SQLite 与缓存会话
        runCatching { CoreSession.get(this) }

        setContent {
            ThemeLiteBalance {
                val nav = rememberNavController()
                val backStack by nav.currentBackStackEntryAsState()
                val currentRoute = backStack?.destination?.route ?: "dashboard"

                val navItems = listOf(
                    NavigationItem("dashboard", "今日", Icons.Rounded.WbSunny),
                    NavigationItem("food", "打卡", Icons.Rounded.Restaurant),
                    NavigationItem("plan", "规划", Icons.Rounded.Insights),
                    NavigationItem("activity", "活力", Icons.Rounded.DirectionsRun),
                    NavigationItem("hub", "健康汇", Icons.Rounded.GridView),
                )

                Scaffold(
                    bottomBar = {
                        // 苹果 iPhone 悬浮液态玻璃底栏 (Floating Liquid Glass Island)
                        FloatingLiquidGlassNavBar(
                            items = navItems,
                            currentRoute = currentRoute,
                            onItemClick = { route ->
                                if (currentRoute != route) {
                                    nav.navigate(route) {
                                        popUpTo("dashboard") { saveState = true }
                                        launchSingleTop = true
                                        restoreState = true
                                    }
                                }
                            },
                        )
                    },
                ) { innerPadding ->
                    Box(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(innerPadding)
                            .background(MaterialTheme.colorScheme.background),
                    ) {
                        NavHost(
                            navController = nav,
                            startDestination = "dashboard",
                        ) {
                            composable("dashboard") { DashboardScreen() }
                            composable("food") { FoodScreen() }
                            composable("plan") { PlanScreen() }
                            composable("activity") { ActivityScreen() }
                            composable("hub") { HealthHubScreen() }
                        }
                    }
                }
            }
        }
    }
}

/**
 * 苹果 iPhone 悬浮液态玻璃底栏 (Floating Liquid Glass Island)
 */
@Composable
private fun FloatingLiquidGlassNavBar(
    items: List<NavigationItem>,
    currentRoute: String,
    onItemClick: (String) -> Unit,
) {
    val isDark = isSystemInDarkTheme()
    val shape = RoundedCornerShape(32.dp)

    Box(
        modifier = Modifier
            .fillMaxWidth()
            .navigationBarsPadding()
            .padding(horizontal = 16.dp, vertical = 10.dp),
        contentAlignment = Alignment.Center,
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                // 液态微光漫晕
                .shadow(
                    elevation = 12.dp,
                    shape = shape,
                    ambientColor = Color(0x1F000000),
                    spotColor = Color(0x14000000),
                )
                .clip(shape)
                // 半透明液态玻璃质感
                .background(
                    Brush.verticalGradient(
                        if (isDark) {
                            listOf(Color(0xE6252830), Color(0xCC1A1C22))
                        } else {
                            listOf(Color(0xF0FFFFFF), Color(0xE0F1F5F9))
                        },
                    ),
                )
                // 顶部反射镜面高光线
                .border(
                    width = 1.dp,
                    brush = if (isDark) LiquidGlassTokens.DarkBorder else LiquidGlassTokens.LightBorder,
                    shape = shape,
                )
                .padding(horizontal = 8.dp, vertical = 6.dp),
            horizontalArrangement = Arrangement.SpaceAround,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            items.forEach { item ->
                val selected = currentRoute == item.route
                val activeBgColor by animateColorAsState(
                    targetValue = if (selected) AppleColors.VitalityCoral.copy(alpha = 0.12f) else Color.Transparent,
                    label = "navActiveBg",
                )
                val activeTint by animateColorAsState(
                    targetValue = if (selected) AppleColors.VitalityCoral else MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.65f),
                    label = "navActiveTint",
                )

                Box(
                    modifier = Modifier
                        .clip(RoundedCornerShape(20.dp))
                        .background(activeBgColor)
                        .clickable(
                            interactionSource = remember { MutableInteractionSource() },
                            indication = null,
                        ) { onItemClick(item.route) }
                        .padding(horizontal = 14.dp, vertical = 8.dp),
                    contentAlignment = Alignment.Center,
                ) {
                    Column(
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.spacedBy(2.dp),
                    ) {
                        Icon(
                            imageVector = item.icon,
                            contentDescription = item.label,
                            tint = activeTint,
                            modifier = Modifier.size(23.dp),
                        )
                        Text(
                            text = item.label,
                            style = MaterialTheme.typography.labelSmall.copy(
                                fontWeight = if (selected) FontWeight.Bold else FontWeight.Normal,
                                color = activeTint,
                                fontSize = 10.5.sp,
                            ),
                        )
                    }
                }
            }
        }
    }
}

private data class NavigationItem(
    val route: String,
    val label: String,
    val icon: ImageVector,
)
